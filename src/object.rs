use crate::material::*;
use crate::shared::*;

#[derive(Copy, Clone)]
pub enum HittableType {
    Sphere,
    Other,
}

/// Information of a ray hit
pub struct HitRecord {
    pub point: Point3,
    pub normal: Vec3,
    pub t: f32,
    pub front_face: bool,
    pub material: Arc<dyn Material>,
}

impl HitRecord {
    pub fn new(ray: Ray, t: f32, outward_normal: Vec3, material: Arc<dyn Material>) -> Self {
        let front_face = ray.direction.dot(outward_normal) < 0.0;
        let normal = if front_face {
            outward_normal
        } else {
            -outward_normal
        };
        HitRecord {
            point: ray.at(t),
            normal: normal,
            t: t,
            front_face: front_face,
            material: material,
        }
    }
}

/// Bounds for RayHittable
#[derive(Copy, Clone)]
pub struct HittableBounds {
    aabb: AABB,
    node_index: usize,
    pub hittable_index: usize,
    pub hittable_type: HittableType,
}

impl Bounded for HittableBounds {
    fn aabb(&self) -> AABB {
        return self.aabb;
    }
}

impl BHShape for HittableBounds {
    fn set_bh_node_index(&mut self, index: usize) {
        self.node_index = index;
    }

    fn bh_node_index(&self) -> usize {
        self.node_index
    }
}

/// An object in the scene which can be hit with a ray
pub trait RayHittable: Send + Sync {
    // Intersect ray with object
    fn intersect(&self, query: RayQuery) -> Option<HitRecord>;
    // Return bounds
    fn compute_bounds(&self, index: usize) -> HittableBounds;
}

#[derive(Clone)]
pub struct Sphere {
    pub center: Point3,
    pub radius: f32,
    pub material: Arc<dyn Material>,
    pub radius_rcp: f32,
    pub radius_sq: f32,
}

impl Sphere {
    pub fn new(center: Point3, radius: f32, material: &Arc<dyn Material>) -> Self {
        Sphere {
            center: center,
            radius: radius,
            material: material.clone(),
            radius_rcp: 1.0 / radius,
            radius_sq: radius * radius,
        }
    }
}

pub struct SphereSimd {
    pub center_x: TracePacketType,
    pub center_y: TracePacketType,
    pub center_z: TracePacketType,

    pub radius: TracePacketType,
    pub radius_rcp: TracePacketType,
    pub radius_sq: TracePacketType,

    pub indices: TracePacketTypeIndex,
}

impl SphereSimd {
    pub fn from_vec(spheres: Vec<Sphere>, indices: Vec<u32>) -> Self {
        SphereSimd {
            center_x: simd_from_fn(&spheres, |s| s.center.x),
            center_y: simd_from_fn(&spheres, |s| s.center.y),
            center_z: simd_from_fn(&spheres, |s| s.center.z),
            radius: simd_from_fn(&spheres, |s| s.radius),
            radius_rcp: simd_from_fn(&spheres, |s| s.radius_rcp),
            radius_sq: simd_from_fn(&spheres, |s| s.radius_sq),
            indices: TracePacketTypeIndex::new(indices[0], indices[1], indices[2], indices[3]),
        }
    }

    pub fn intersect_packet(&self, packet: &RayPacket) -> TracePacketType {
        let oc_x = packet.ray_origin_x - self.center_x;
        let oc_y = packet.ray_origin_y - self.center_y;
        let oc_z = packet.ray_origin_z - self.center_z;

        let a = packet.direction_length_squared;

        let half_b = oc_x * packet.ray_direction_x
            + oc_y * packet.ray_direction_y
            + oc_z * packet.ray_direction_z;

        let oc_length_squared = oc_x * oc_x + oc_y * oc_y + oc_z * oc_z;
        let c = oc_length_squared - self.radius_sq;

        let discriminant = half_b * half_b - a * c;
        let discriminant_mask = discriminant.gt(TracePacketType::splat(0.0));

        let sqrtd = discriminant.sqrt();
        let root_a = (-half_b - sqrtd) / a;
        let root_b = (-half_b + sqrtd) / a;

        let root = root_a.min(root_b);

        let root_mask_a = root_a.gt(packet.ray_t_min) & (root_a.lt(packet.ray_t_max));
        let root_mask_b = root_b.gt(packet.ray_t_min) & (root_b.lt(packet.ray_t_max));
        let hit_mask = (root_mask_a | root_mask_b) & discriminant_mask;

        let miss = TracePacketType::MAX;

        return hit_mask.select(root, miss);
    }
}

impl RayHittable for Sphere {
    fn intersect(&self, query: RayQuery) -> Option<HitRecord> {
        let r = query.ray;
        let oc = r.origin - self.center;
        let a = r.direction_length_squared;
        let half_b = oc.dot(r.direction);
        let c = oc.length_squared() - self.radius_sq;
        let discriminant = half_b * half_b - a * c;
        if discriminant < 0.0 {
            return None;
        }

        let sqrtd = discriminant.sqrt();

        let mut root = (-half_b - sqrtd) / a;
        if root < query.t_min || query.t_max < root {
            root = (-half_b + sqrtd) / a;
            if root < query.t_min || query.t_max < root {
                return None;
            }
        }

        let t = root;
        let point = r.at(t);
        let outward_normal = (point - self.center) * self.radius_rcp;
        let record = HitRecord::new(r, t, outward_normal, self.material.clone());

        return Some(record);
    }

    fn compute_bounds(&self, hittable_index: usize) -> HittableBounds {
        let half_size = Vec3::new(self.radius, self.radius, self.radius);
        let min = self.center - half_size;
        let max = self.center + half_size;
        let aabb = AABB::with_bounds(min, max);

        HittableBounds {
            aabb,
            node_index: 0,
            hittable_index: hittable_index,
            hittable_type: HittableType::Sphere,
        }
    }
}
