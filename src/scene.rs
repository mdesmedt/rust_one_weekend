use crate::object::*;
use crate::shared::*;

use bvh::bvh::Bvh;

/// Basic scene which holds objects and a BVH
pub struct Scene {
    // List of hittables
    pub objects: Vec<Box<dyn RayHittable>>,

    // List of bounds for hittables
    pub bounds: Vec<HittableBounds>,

    // Acceleration structure
    pub bvh: Option<Bvh<f32, 3>>,
}

impl Scene {
    pub fn new() -> Self {
        Scene {
            objects: Vec::new(),
            bounds: Vec::new(),
            bvh: None,
        }
    }

    pub fn build_bvh(&mut self) {
        // Compute bounds
        for (i, hittable) in self.objects.iter().enumerate() {
            let bounds = hittable.compute_bounds(i);
            self.bounds.push(bounds);
        }
        // Build BVH
        self.bvh = Some(Bvh::build(&mut self.bounds));
    }

    /// Return the closest intersection (or None) in the scene using the ray
    pub fn intersect(&self, mut query: RayQuery) -> Option<HitRecord> {
        let mut closest_hit_option: Option<HitRecord> = None;

        if let Some(bvh) = &self.bvh {
            // Traverse the BVH
            let origin = point_to_nalgebra(query.ray.origin);
            let direction = vec_to_nalgebra(query.ray.direction);
            let bvh_ray = bvh::ray::Ray::new(origin, direction);
            let hit_bounds = bvh.traverse_iterator(&bvh_ray, &self.bounds);

            // Iterate over hit objects to find closest
            for bounds in hit_bounds {
                let obj = self.objects[bounds.hittable_index].as_ref();
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
        closest_hit_option
    }
}
