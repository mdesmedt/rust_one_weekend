use crate::object::*;
use crate::shared::*;

use bvh::bvh::{BVHNode, BVH};

/// Custom iterator to replaces BVH::traverse without memory allocations
pub struct BVHIterator<'a> {
    bvh: &'a BVH,
    ray: bvh::ray::Ray,
    stack: [usize; 32],
    current_node: usize,
    current_stack: usize,
    has_node: bool,
}

impl<'a> BVHIterator<'a> {
    pub fn new(bvh: &'a BVH, ray: bvh::ray::Ray) -> Self {
        BVHIterator {
            bvh: bvh,
            ray: ray,
            stack: [0; 32],
            current_node: 0,
            current_stack: 0,
            has_node: true,
        }
    }

    fn stack_empty(&self) -> bool {
        return self.current_stack == 0;
    }

    fn stack_push(&mut self, node: usize) {
        self.stack[self.current_stack] = node;
        self.current_stack += 1;
    }

    fn stack_pop(&mut self) -> usize {
        self.current_stack -= 1;
        return self.stack[self.current_stack];
    }

    fn move_left(&mut self) {
        match self.bvh.nodes[self.current_node] {
            BVHNode::Node {
                child_l_index,
                ref child_l_aabb,
                ..
            } => {
                if self.ray.intersects_aabb(child_l_aabb) {
                    self.current_node = child_l_index;
                } else {
                    self.has_node = false;
                }
            }
            BVHNode::Leaf { .. } => {
                self.has_node = false;
            }
        }
    }

    fn move_right(&mut self) {
        match self.bvh.nodes[self.current_node] {
            BVHNode::Node {
                child_r_index,
                ref child_r_aabb,
                ..
            } => {
                if self.ray.intersects_aabb(child_r_aabb) {
                    self.current_node = child_r_index;
                } else {
                    self.has_node = false;
                }
            }
            BVHNode::Leaf { .. } => {
                self.has_node = false;
            }
        }
    }
}

impl<'a> Iterator for BVHIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        loop {
            if self.stack_empty() && !self.has_node {
                break;
            }
            if self.has_node {
                self.stack_push(self.current_node);
                self.move_left();
            } else {
                self.current_node = self.stack_pop();
                match self.bvh.nodes[self.current_node] {
                    BVHNode::Node { .. } => {
                        self.has_node = true;
                        self.move_right();
                    }
                    BVHNode::Leaf { shape_index, .. } => {
                        self.has_node = false;
                        return Some(shape_index);
                    }
                }
            }
        }
        return None;
    }
}

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

            let bvh_iterator = BVHIterator::new(bvh, bvh_ray);

            for shape_idx in bvh_iterator {
                let obj_boxed = &self.objects[shape_idx];
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
