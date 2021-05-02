use crate::material::*;
use crate::shared::*;

use rtbvh::*;

/// Information of a ray hit
pub struct HitRecord {
    pub point: Point3,
    pub normal: Vec3,
    pub t: f32,
    pub front_face: bool,
    pub material: Arc<dyn Material>,
}

impl HitRecord {
    pub fn new(ray: crate::shared::Ray, t: f32, outward_normal: Vec3, material: Arc<dyn Material>) -> Self {
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
#[derive(Debug, Copy, Clone)]
pub struct HittableBounds {
    aabb: Aabb,
    node_index: usize,
    pub hittable_index: usize,
}

impl Primitive for HittableBounds {
    fn center(&self) -> Vec3 {
        self.aabb.center()
    }

    fn aabb(&self) -> Aabb {
        self.aabb
    }
}

/// An object in the scene which can be hit with a ray
pub trait RayHittable: Send + Sync {
    // Intersect ray with object
    fn intersect(&self, query: RayQuery) -> Option<HitRecord>;
    // bounds
    fn compute_bounds(&self, index: usize) -> HittableBounds;
}

pub struct Sphere {
    pub center: Point3,
    pub radius: f32,
    pub material: Arc<dyn Material>,
    radius_rcp: f32,
    radius_sq: f32,
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

impl RayHittable for Sphere {
    fn intersect(&self, query: RayQuery) -> Option<HitRecord> {
        let r = query.ray;
        let oc = r.origin - self.center;
        let a = r.direction.length_squared();
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

    fn compute_bounds(&self, hittable_index: usize) -> HittableBounds
    {
        let half_size = Vec3::new(self.radius, self.radius, self.radius);
        let min = self.center - half_size;
        let max = self.center + half_size;
        let mut aabb = Aabb::new();
        aabb.grow(min);
        aabb.grow(max);

        HittableBounds {
            aabb,
            node_index: 0,
            hittable_index: hittable_index
        }
    }
}
