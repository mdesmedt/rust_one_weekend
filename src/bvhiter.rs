use bvh::bvh::{BVHNode, BVH};

/// Custom iterator to replaces BVH::traverse without memory allocations
pub struct BVHIterator<'a> {
    bvh: &'a BVH,
    ray: bvh::ray::Ray,
    stack: [usize; 32],
    node_index: usize,
    stack_size: usize,
    has_node: bool,
}

impl<'a> BVHIterator<'a> {
    pub fn new(bvh: &'a BVH, ray: bvh::ray::Ray) -> Self {
        BVHIterator {
            bvh: bvh,
            ray: ray,
            stack: [0; 32], // 4 billion items seems enough?
            node_index: 0,
            stack_size: 0,
            has_node: true, // Whether or not we have a valid node (or leaf)
        }
    }

    /// Test if stack is empty.
    fn is_stack_empty(&self) -> bool {
        return self.stack_size == 0;
    }

    /// Push node onto stack. Not guarded against overflow.
    fn stack_push(&mut self, node: usize) {
        self.stack[self.stack_size] = node;
        self.stack_size += 1;
    }

    /// Pop the stack and return the node. Not guarded against underflow.
    fn stack_pop(&mut self) -> usize {
        self.stack_size -= 1;
        return self.stack[self.stack_size];
    }

    /// Attempt to move to the left child of the current node.
    fn move_left(&mut self) {
        match self.bvh.nodes[self.node_index] {
            BVHNode::Node {
                child_l_index,
                ref child_l_aabb,
                ..
            } => {
                if self.ray.intersects_aabb(child_l_aabb) {
                    self.node_index = child_l_index;
                    self.has_node = true;
                } else {
                    self.has_node = false;
                }
            }
            BVHNode::Leaf { .. } => {
                self.has_node = false;
            }
        }
    }

    /// Attempt to move to the right child of the current node.
    fn move_right(&mut self) {
        match self.bvh.nodes[self.node_index] {
            BVHNode::Node {
                child_r_index,
                ref child_r_aabb,
                ..
            } => {
                if self.ray.intersects_aabb(child_r_aabb) {
                    self.node_index = child_r_index;
                    self.has_node = true;
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
            if self.is_stack_empty() && !self.has_node {
                // Completed traversal.
                break;
            }
            if self.has_node {
                // If we have any node, save it and attempt to move to its left child.
                self.stack_push(self.node_index);
                self.move_left();
            } else {
                // Go back up the stack and see if a node or leaf was pushed.
                self.node_index = self.stack_pop();
                match self.bvh.nodes[self.node_index] {
                    BVHNode::Node { .. } => {
                        // If a node was pushed, now attempt to move to its right child.
                        self.move_right();
                    }
                    BVHNode::Leaf { shape_index, .. } => {
                        // We previously pushed a leaf node. This is the "visit" of the in-order traverse.
                        // Next time we call next we try to pop the stack again.
                        self.has_node = false;
                        return Some(shape_index);
                    }
                }
            }
        }
        return None;
    }
}
