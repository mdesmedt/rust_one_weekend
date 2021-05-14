use crate::object::*;
use crate::shared::*;

use bvh::bvh::BVH;

/// Basic scene which holds objects and a BVH
pub struct Scene {
    // List of generic hittables
    pub objects_other: Vec<Box<dyn RayHittable>>,

    // List of spheres
    pub objects_sphere: Vec<Sphere>,

    // List of SIMD spheres
    pub simd_sphere: Vec<SphereSimd>,

    // List of bounds for hittables
    pub bounds: Vec<HittableBounds>,

    // Acceleration structure
    pub bvh: Option<BVH>,
}

impl Scene {
    pub fn new() -> Self {
        Scene {
            objects_other: Vec::new(),
            objects_sphere: Vec::new(),
            simd_sphere: Vec::new(),
            bounds: Vec::new(),
            bvh: None,
        }
    }

    pub fn add_sphere(&mut self, s: Sphere) {
        self.objects_sphere.push(s);
    }

    fn build_simd(&mut self) {
        let mut index_start: u32 = 0;
        let packet_size = TRACE_PACKET_SIZE as u32;
        for chunk in self.objects_sphere.chunks(TRACE_PACKET_SIZE) {
            let indices = (index_start..index_start + packet_size).collect();
            let simd = SphereSimd::from_vec(chunk.to_vec(), indices);
            self.simd_sphere.push(simd);
            index_start += packet_size;
        }
    }

    pub fn build_scene(&mut self) {
        // Build SIMD vector
        self.build_simd();
        // Compute bounds
        for (i, hittable) in self.objects_other.iter().enumerate() {
            self.bounds.push(hittable.compute_bounds(i));
        }
        for (i, sphere) in self.objects_sphere.iter().enumerate() {
            self.bounds.push(sphere.compute_bounds(i));
        }
        // Build BVH
        self.bvh = Some(BVH::build(&mut self.bounds));
    }

    /// Return the closest intersection (or None) in the scene using the ray
    pub fn intersect(&self, mut query: RayQuery) -> Option<HitRecord> {
        let mut closest_hit_option: Option<HitRecord> = None;

        if let Some(bvh) = &self.bvh {
            // Traverse the BVH
            let bvh_ray = bvh::ray::Ray::new(query.ray.origin, query.ray.direction);
            let hit_bounds = bvh.traverse_iterator(&bvh_ray, &self.bounds);

            // Iterate over hit objects to find closest
            for bounds in hit_bounds {
                let obj = &self.objects_sphere[bounds.hittable_index];
                let hit_option = obj.intersect(query);
                if hit_option.is_some() {
                    // Shorten the ray
                    query.t_max = f32::min(query.t_max, hit_option.as_ref().unwrap().t);
                }
                if closest_hit_option.is_none() {
                    closest_hit_option = hit_option;
                } else if hit_option.is_some() {
                    let closest_hit = closest_hit_option.as_ref().unwrap();
                    let hit = hit_option.as_ref().unwrap();
                    if hit.t < closest_hit.t {
                        closest_hit_option = hit_option;
                    }
                }
            }
        }
        return closest_hit_option;
    }

    pub fn intersect_packet(&self, packet: &RayPacket) -> [Option<HitRecord>; TRACE_PACKET_SIZE] {
        let mut min_t: TracePacketType = TracePacketType::MAX;
        let mut indices: TracePacketTypeIndex = TracePacketTypeIndex::splat(0);
        for sphere_simd in &self.simd_sphere {
            let packet_t = sphere_simd.intersect_packet(packet);
            let hit_mask = packet_t.ne(TracePacketType::MAX);
            min_t = min_t.min(packet_t);
            indices = hit_mask.select(sphere_simd.indices, indices);
        }
        let hit_mask = min_t.ne(TracePacketType::MAX);
        <[Option<HitRecord>; TRACE_PACKET_SIZE]>::init_with_indices(|i| {
            let is_hit = hit_mask.extract(i);
            let t = min_t.extract(i);
            let index = indices.extract(i) as usize;
            if is_hit {
                let ray = packet.rays[i];
                let point = ray.at(t);
                let sphere = &self.objects_sphere[index];
                let outward_normal = (point - sphere.center) * sphere.radius_rcp;
                Some(HitRecord::new(
                    ray,
                    t,
                    outward_normal,
                    sphere.material.clone(),
                ))
            } else {
                None
            }
        })
    }
}
