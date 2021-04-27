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
