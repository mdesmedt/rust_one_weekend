use crate::object::*;
use crate::shared::*;

use bvh::bvh::BVH;

/// Basic scene which holds objects and a BVH
pub struct Scene {
    pub objects: Vec<Box<dyn RayHittable>>,
    pub bvh: Option<BVH>,
}

impl Scene {
    pub fn new() -> Self {
        Scene {
            objects: Vec::new(),
            bvh: None,
        }
    }

    pub fn build_bvh(&mut self) {
        self.bvh = Some(BVH::build(&mut self.objects));
    }

    /// Return the closest intersection (or None) in the scene using the ray
    pub fn intersect(&self, mut query: RayQuery) -> Option<HitRecord> {
        let mut closest_hit_option: Option<HitRecord> = None;

        if let Some(bvh) = &self.bvh {
            // Traverse the BVH
            let bvh_ray = bvh::ray::Ray::new(
                query.ray.origin.to_nalgebra_point(),
                query.ray.direction.to_nalgebra_vector(),
            );
            let hit_objects = bvh.traverse_iterator(&bvh_ray, &self.objects);

            // Iterate over hit objects to find closest
            for obj_boxed in hit_objects {
                let obj = obj_boxed.as_ref();
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
}
