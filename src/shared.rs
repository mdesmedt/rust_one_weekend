pub use bvh::aabb::{Aabb, Bounded};
pub use bvh::bounding_hierarchy::{BHShape, BoundingHierarchy};
pub use glam::Vec3;
use rand::Rng;
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro128Plus;
pub use std::sync::Arc;

pub type Point3 = glam::Vec3;
pub type Color = glam::Vec3;
pub type ColorDisplay = u32;

pub const TRACE_EPSILON: f32 = 0.001;
pub const TRACE_INFINITY: f32 = f32::MAX;

pub fn index_from_xy(image_width: u32, _image_height: u32, x: u32, y: u32) -> usize {
    (y * image_width + x) as usize
}

/// A minimal ray
#[derive(Copy, Clone)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray { origin, direction }
    }

    pub fn at(&self, t: f32) -> Point3 {
        self.origin + t * self.direction
    }
}

/// A RayQuery for intersection
#[derive(Copy, Clone)]
pub struct RayQuery {
    pub ray: Ray,
    pub t_min: f32,
    pub t_max: f32,
}

pub fn u8_vec_from_color_display(c: ColorDisplay) -> Vec<u8> {
    let b = c as u8;
    let g = (c >> 8) as u8;
    let r = (c >> 16) as u8;
    vec![r, g, b]
}

pub fn color_display_from_u8_rgb(r: u8, g: u8, b: u8) -> ColorDisplay {
    let (r, g, b) = (r as u32, g as u32, b as u32);
    (r << 16) | (g << 8) | b
}

pub fn color_display_from_f32_rgb(r: f32, g: f32, b: f32) -> ColorDisplay {
    color_display_from_u8_rgb((255.0 * r) as u8, (255.0 * g) as u8, (255.0 * b) as u8)
}

pub fn color_display_from_render(c: Color) -> ColorDisplay {
    let gamma = 1.0 / 2.2;
    let col_gamma = Color::new(c.x.powf(gamma), c.y.powf(gamma), c.z.powf(gamma));
    color_display_from_f32_rgb(col_gamma.x, col_gamma.y, col_gamma.z)
}

pub fn degrees_to_radians(degrees: f32) -> f32 {
    degrees * std::f32::consts::PI / 180.0
}

pub struct RayRng {
    rng: Xoshiro128Plus,
}

impl RayRng {
    pub fn new(seed: u64) -> RayRng {
        RayRng {
            rng: Xoshiro128Plus::seed_from_u64(seed),
        }
    }

    pub fn gen_range(&mut self, range: std::ops::Range<f32>) -> f32 {
        self.rng.random_range(range)
    }
}

pub fn vec3_random_range(rng: &mut RayRng, range: std::ops::Range<f32>) -> Vec3 {
    Vec3::new(
        rng.gen_range(range.clone()),
        rng.gen_range(range.clone()),
        rng.gen_range(range),
    )
}

#[allow(dead_code)]
pub fn vec3_random(rng: &mut RayRng) -> Vec3 {
    vec3_random_range(rng, 0.0..1.0)
}

pub fn random_in_unit_sphere(rng: &mut RayRng) -> Vec3 {
    loop {
        let p = vec3_random_range(rng, -1.0..1.0);
        if p.length_squared() < 1.0 {
            return p;
        }
    }
}

pub fn random_unit_vector(rng: &mut RayRng) -> Vec3 {
    random_in_unit_sphere(rng).normalize()
}

#[allow(dead_code)]
pub fn random_in_hemisphere(rng: &mut RayRng, normal: Vec3) -> Vec3 {
    let in_unit_sphere = random_in_unit_sphere(rng);
    if in_unit_sphere.dot(normal) > 0.0 {
        in_unit_sphere // In the same hemisphere as the normal
    } else {
        -in_unit_sphere
    }
}

/// Vec3 extensions
pub trait VecExt {
    fn near_zero(&self) -> bool;
}

impl VecExt for Vec3 {
    /// Are all components near zero
    fn near_zero(&self) -> bool {
        let s = 1e-8;
        (self.x.abs() < s) && (self.y.abs() < s) && (self.z.abs() < s)
    }
}

pub fn vec_reflect(v: Vec3, n: Vec3) -> Vec3 {
    v - 2.0 * v.dot(n) * n
}

pub fn vec_refract(uv: Vec3, n: Vec3, etai_over_etat: f32) -> Vec3 {
    let cos_theta = f32::min((-uv).dot(n), 1.0);
    let r_out_perp = etai_over_etat * (uv + cos_theta * n);
    let r_out_parallel = -f32::sqrt(f32::abs(1.0 - r_out_perp.length_squared())) * n;
    r_out_perp + r_out_parallel
}

pub fn reflectance(cosine: f32, ref_idx: f32) -> f32 {
    // Use Schlick's approximation for reflectance.
    let mut r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    r0 = r0 * r0;
    r0 + (1.0 - r0) * ((1.0 - cosine).powf(5.0))
}

pub fn ceil_div(x: u32, y: u32) -> u32 {
    (x + y - 1) / y
}

pub fn random_in_unit_disk(rng: &mut RayRng) -> Vec3 {
    loop {
        let p = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), 0.0);
        if p.length_squared() < 1.0 {
            return p;
        }
    }
}

pub fn color_random(rng: &mut RayRng) -> Color {
    color_random_range(rng, 0.0..1.0)
}

pub fn color_random_range(rng: &mut RayRng, range: std::ops::Range<f32>) -> Color {
    Color::new(
        rng.gen_range(range.clone()),
        rng.gen_range(range.clone()),
        rng.gen_range(range),
    )
}

pub fn point_to_nalgebra(p: Point3) -> nalgebra::Point3<f32> {
    return nalgebra::Point3::new(p.x, p.y, p.z);
}

pub fn vec_to_nalgebra(p: Vec3) -> nalgebra::Vector3<f32> {
    return nalgebra::Vector3::new(p.x, p.y, p.z);
}
