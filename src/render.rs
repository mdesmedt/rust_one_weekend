use crate::camera::*;
use crate::scene::*;
use crate::shared::*;

/// Recursive ray tracing
fn ray_color(rng: &mut RayRng, ray: Ray, scene: &Scene, depth: i32, ray_count: &mut u32) -> Color {
    if depth <= 0 {
        return Color::ZERO;
    }

    // Intersect scene
    let query = RayQuery {
        ray,
        t_min: TRACE_EPSILON,
        t_max: TRACE_INFINITY,
    };
    let hit_option = scene.intersect(query);
    *ray_count += 1;

    // If we hit something
    if let Some(hit) = hit_option {
        let scatter_option = hit.material.scatter(rng, &ray, &hit);

        // Recurse
        if let Some(scatter) = scatter_option {
            return scatter.attenuation
                * ray_color(rng, scatter.scattered_ray, scene, depth - 1, ray_count);
        }

        return Color::ZERO;
    }

    // Background
    let unit_direction = ray.direction.normalize();
    let t = 0.5 * (unit_direction.y + 1.0);
    (1.0 - t) * Color::new(1.0, 1.0, 1.0) + t * Color::new(0.5, 0.7, 1.0)
}

/// Renderer which generates pixels using the scene and camera
pub struct Renderer {
    image_width: u32,
    image_height: u32,
    scene: Scene,
    camera: Camera,
    samples_per_pixel: u32,
    max_depth: i32,
}

impl Renderer {
    pub fn new(
        image_width: u32,
        image_height: u32,
        samples_per_pixel: u32,
        scene: Scene,
        camera: Camera,
    ) -> Self {
        Renderer {
            image_width,
            image_height,
            scene,
            camera,
            samples_per_pixel,
            max_depth: 50,
        }
    }

    pub fn render_pixel(&self, x: u32, y: u32) -> Color {
        let mut rng = RayRng::new(0);

        // Set up supersampling
        let mut color_accum = Color::ZERO;
        let u_base = x as f32 / (self.image_width as f32 - 1.0);
        let v_base = (self.image_height - y - 1) as f32 / (self.image_height as f32 - 1.0);
        let u_rand = 1.0 / (self.image_width as f32 - 1.0);
        let v_rand = 1.0 / (self.image_height as f32 - 1.0);

        let mut ray_count = 0;

        // Supersample this pixel
        for _ in 0..self.samples_per_pixel {
            let u = u_base + rng.gen_range(0.0..u_rand);
            let v = v_base + rng.gen_range(0.0..v_rand);
            let ray = self.camera.get_ray(&mut rng, u, v);
            // Start the primary here from here
            color_accum += ray_color(&mut rng, ray, &self.scene, self.max_depth, &mut ray_count);
        }
        color_accum /= self.samples_per_pixel as f32;

        color_accum
    }
}
