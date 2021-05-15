pub use bvh::aabb::{Bounded, AABB};
pub use bvh::bounding_hierarchy::{BHShape, BoundingHierarchy};
pub use glam::Vec3;
pub use init_with::InitWith;
pub use packed_simd::*;
pub use rand::Rng;
pub use std::sync::Arc;

pub type Point3 = glam::Vec3;
pub type Color = glam::Vec3;
pub type ColorDisplay = u32;

pub const TRACE_EPSILON: f32 = 0.001;
pub const TRACE_INFINITY: f32 = f32::MAX;

pub const TRACE_PACKET: bool = true;
pub const TRACE_PACKET_SIZE: usize = 4;
pub type TracePacketType = f32x4;
pub type TracePacketTypeMask = m8x4;
pub type TracePacketTypeIndex = u32x4;

pub fn index_from_xy(image_width: u32, _image_height: u32, x: u32, y: u32) -> usize {
    (y * image_width + x) as usize
}

/// A minimal ray
#[derive(Copy, Clone)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub direction_length_squared: f32,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray {
            origin,
            direction,
            direction_length_squared: direction.length_squared(),
        }
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

pub struct RayPacket {
    pub ray_origin_x: TracePacketType,
    pub ray_origin_y: TracePacketType,
    pub ray_origin_z: TracePacketType,

    pub ray_direction_x: TracePacketType,
    pub ray_direction_y: TracePacketType,
    pub ray_direction_z: TracePacketType,

    pub ray_t_min: TracePacketType,
    pub ray_t_max: TracePacketType,

    pub direction_length_squared: TracePacketType,

    pub mask: TracePacketTypeMask,

    pub rays: [Ray; TRACE_PACKET_SIZE],
    pub is_ray_live: [bool; TRACE_PACKET_SIZE],
    pub ray_live_count: usize,
}

impl RayPacket {
    pub fn new(rays: [Ray; TRACE_PACKET_SIZE]) -> Self {
        RayPacket {
            ray_live_count: TRACE_PACKET_SIZE,
            is_ray_live: [true; TRACE_PACKET_SIZE],
            rays: rays.clone(),
            mask: TracePacketTypeMask::new(
                true, true, true,
                true,
                //true, true, true, true, true, true, true, true, true, true, true, true,
            ),
            ray_t_min: TracePacketType::splat(TRACE_EPSILON),
            ray_t_max: TracePacketType::splat(TRACE_INFINITY),
            ray_origin_x: TracePacketType::new(
                rays[0].origin.x,
                rays[1].origin.x,
                rays[2].origin.x,
                rays[3].origin.x,
                // rays[4].origin.x,
                // rays[5].origin.x,
                // rays[6].origin.x,
                // rays[7].origin.x,
                // rays[8].origin.x,
                // rays[9].origin.x,
                // rays[10].origin.x,
                // rays[11].origin.x,
                // rays[12].origin.x,
                // rays[13].origin.x,
                // rays[14].origin.x,
                // rays[15].origin.x,
            ),
            ray_origin_y: TracePacketType::new(
                rays[0].origin.y,
                rays[1].origin.y,
                rays[2].origin.y,
                rays[3].origin.y,
                // rays[4].origin.y,
                // rays[5].origin.y,
                // rays[6].origin.y,
                // rays[7].origin.y,
                // rays[8].origin.y,
                // rays[9].origin.y,
                // rays[10].origin.y,
                // rays[11].origin.y,
                // rays[12].origin.y,
                // rays[13].origin.y,
                // rays[14].origin.y,
                // rays[15].origin.y,
            ),
            ray_origin_z: TracePacketType::new(
                rays[0].origin.z,
                rays[1].origin.z,
                rays[2].origin.z,
                rays[3].origin.z,
                // rays[4].origin.z,
                // rays[5].origin.z,
                // rays[6].origin.z,
                // rays[7].origin.z,
                // rays[8].origin.z,
                // rays[9].origin.z,
                // rays[10].origin.z,
                // rays[11].origin.z,
                // rays[12].origin.z,
                // rays[13].origin.z,
                // rays[14].origin.z,
                // rays[15].origin.z,
            ),
            ray_direction_x: TracePacketType::new(
                rays[0].direction.x,
                rays[1].direction.x,
                rays[2].direction.x,
                rays[3].direction.x,
                // rays[4].direction.x,
                // rays[5].direction.x,
                // rays[6].direction.x,
                // rays[7].direction.x,
                // rays[8].direction.x,
                // rays[9].direction.x,
                // rays[10].direction.x,
                // rays[11].direction.x,
                // rays[12].direction.x,
                // rays[13].direction.x,
                // rays[14].direction.x,
                // rays[15].direction.x,
            ),
            ray_direction_y: TracePacketType::new(
                rays[0].direction.y,
                rays[1].direction.y,
                rays[2].direction.y,
                rays[3].direction.y,
                // rays[4].direction.y,
                // rays[5].direction.y,
                // rays[6].direction.y,
                // rays[7].direction.y,
                // rays[8].direction.y,
                // rays[9].direction.y,
                // rays[10].direction.y,
                // rays[11].direction.y,
                // rays[12].direction.y,
                // rays[13].direction.y,
                // rays[14].direction.y,
                // rays[15].direction.y,
            ),
            ray_direction_z: TracePacketType::new(
                rays[0].direction.z,
                rays[1].direction.z,
                rays[2].direction.z,
                rays[3].direction.z,
                // rays[4].direction.z,
                // rays[5].direction.z,
                // rays[6].direction.z,
                // rays[7].direction.z,
                // rays[8].direction.z,
                // rays[9].direction.z,
                // rays[10].direction.z,
                // rays[11].direction.z,
                // rays[12].direction.z,
                // rays[13].direction.z,
                // rays[14].direction.z,
                // rays[15].direction.z,
            ),
            direction_length_squared: TracePacketType::new(
                rays[0].direction_length_squared,
                rays[1].direction_length_squared,
                rays[2].direction_length_squared,
                rays[3].direction_length_squared,
            ),
        }
    }

    pub fn update_ray(&mut self, i: usize, ray: Ray) {
        self.ray_origin_x = self.ray_origin_x.replace(i, ray.origin.x);
        self.ray_origin_y = self.ray_origin_y.replace(i, ray.origin.y);
        self.ray_origin_z = self.ray_origin_z.replace(i, ray.origin.z);
        self.ray_direction_x = self.ray_direction_x.replace(i, ray.direction.x);
        self.ray_direction_y = self.ray_direction_y.replace(i, ray.direction.y);
        self.ray_direction_z = self.ray_direction_z.replace(i, ray.direction.z);
        self.direction_length_squared = self.direction_length_squared.replace(i, ray.direction_length_squared);
        self.rays[i] = ray;
    }

    pub fn end_ray(&mut self, i: usize) {
        self.mask = self.mask.replace(i, false);
        self.is_ray_live[i] = false;
        self.ray_live_count -= 1;
    }
}

pub fn simd_from_fn<T>(v: &Vec<T>, f: fn(&T) -> f32) -> TracePacketType {
    let mut vals: Vec<f32> = v.iter().map(f).collect();
    while vals.len() < TRACE_PACKET_SIZE {
        vals.push(0.0);
    }
    TracePacketType::new(vals[0], vals[1], vals[2], vals[3])
}

pub fn u8_vec_from_color_display(c: ColorDisplay) -> Vec<u8> {
    let b = c as u8;
    let g = (c >> 8) as u8;
    let r = (c >> 16) as u8;
    return vec![r, g, b];
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
    return degrees * std::f32::consts::PI / 180.0;
}

pub fn vec3_random_range(min: f32, max: f32) -> Vec3 {
    let mut rng = rand::thread_rng();
    return Vec3::new(
        rng.gen_range(min..max),
        rng.gen_range(min..max),
        rng.gen_range(min..max),
    );
}

#[allow(dead_code)]
pub fn vec3_random() -> Vec3 {
    return vec3_random_range(0.0, 1.0);
}

pub fn random_in_unit_sphere() -> Vec3 {
    loop {
        let p = vec3_random_range(-1.0, 1.0);
        if p.length_squared() < 1.0 {
            return p;
        }
    }
}

pub fn random_unit_vector() -> Vec3 {
    return random_in_unit_sphere().normalize();
}

#[allow(dead_code)]
pub fn random_in_hemisphere(normal: Vec3) -> Vec3 {
    let in_unit_sphere = random_in_unit_sphere();
    if in_unit_sphere.dot(normal) > 0.0 {
        return in_unit_sphere; // In the same hemisphere as the normal
    } else {
        return -in_unit_sphere;
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
    return r_out_perp + r_out_parallel;
}

pub fn reflectance(cosine: f32, ref_idx: f32) -> f32 {
    // Use Schlick's approximation for reflectance.
    let mut r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    r0 = r0 * r0;
    return r0 + (1.0 - r0) * ((1.0 - cosine).powf(5.0));
}

pub fn ceil_div(x: u32, y: u32) -> u32 {
    (x + y - 1) / y
}

pub fn random_in_unit_disk() -> Vec3 {
    let mut rng = rand::thread_rng();
    loop {
        let p = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), 0.0);
        if p.length_squared() < 1.0 {
            return p;
        }
    }
}

pub fn color_random<T: Rng>(rng: &mut T) -> Color {
    color_random_range(rng, 0.0, 1.0)
}

pub fn color_random_range<T: Rng>(rng: &mut T, min: f32, max: f32) -> Color {
    return Color::new(
        rng.gen_range(min..max),
        rng.gen_range(min..max),
        rng.gen_range(min..max),
    );
}
