use crate::object::*;
use crate::shared::*;

/// Result of Material::scatter
pub struct ScatterResult {
    pub attenuation: Color,
    pub scattered_ray: Ray,
}

/// A material which can scatter rays
pub trait Material: Send + Sync {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<ScatterResult>;
}

pub struct Lambertian {
    pub albedo: Color,
}

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, hit: &HitRecord) -> Option<ScatterResult> {
        let mut scatter_direction = hit.normal + random_unit_vector();
        if scatter_direction.near_zero() {
            scatter_direction = hit.normal;
        }

        let scattered = Ray::new(hit.point, scatter_direction);
        Some(ScatterResult {
            attenuation: self.albedo,
            scattered_ray: scattered,
        })
    }
}

pub struct Metal {
    pub albedo: Color,
    pub fuzz: f32,
}

impl Material for Metal {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<ScatterResult> {
        let reflected = vec_reflect(ray.direction.normalize(), hit.normal);

        let scattered = Ray::new(hit.point, reflected + self.fuzz * random_in_unit_sphere());
        Some(ScatterResult {
            attenuation: self.albedo,
            scattered_ray: scattered,
        })
    }
}

pub struct Dielectric {
    pub ir: f32,
}

impl Material for Dielectric {
    fn scatter(&self, ray: &Ray, hit: &HitRecord) -> Option<ScatterResult> {
        let mut rng = rand::thread_rng();

        let attenuation = Color::new(1.0, 1.0, 1.0);
        let refraction_ratio = if hit.front_face {
            1.0 / self.ir
        } else {
            self.ir
        };

        let unit_direction = ray.direction.normalize();
        let cos_theta = f32::min((-unit_direction).dot(hit.normal), 1.0);
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let cannot_refract = refraction_ratio * sin_theta > 1.0;
        let direction: Vec3;
        if cannot_refract || reflectance(cos_theta, refraction_ratio) > rng.gen_range(0.0..1.0) {
            direction = vec_reflect(unit_direction, hit.normal);
        } else {
            direction = vec_refract(unit_direction, hit.normal, refraction_ratio);
        }

        let scattered = Ray::new(hit.point, direction);
        Some(ScatterResult {
            attenuation: attenuation,
            scattered_ray: scattered,
        })
    }
}
