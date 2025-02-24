use crate::camera::*;
use crate::scene::*;
use crate::shared::*;
use crate::BufferPacket;
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use rayon::prelude::*;

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
    max_depth: i32
}

impl Renderer {
    pub fn new(
        image_width: u32,
        image_height: u32,
        samples_per_pixel: u32,
        scene: Scene,
        camera: Camera
    ) -> Self {
        Renderer {
            image_width,
            image_height,
            scene,
            camera,
            samples_per_pixel,
            max_depth: 50
        }
    }

    pub fn render_pixel(&self, x: u32, y: u32, rng: &mut RayRng, ray_count: &mut u32) -> Color {
        // Set up supersampling
        let mut color_accum = Color::ZERO;
        let u_base = x as f32 / (self.image_width as f32 - 1.0);
        let v_base = (self.image_height - y - 1) as f32 / (self.image_height as f32 - 1.0);
        let u_rand = 1.0 / (self.image_width as f32 - 1.0);
        let v_rand = 1.0 / (self.image_height as f32 - 1.0);

        // Supersample this pixel
        for _ in 0..self.samples_per_pixel {
            let u = u_base + rng.gen_range(0.0..u_rand);
            let v = v_base + rng.gen_range(0.0..v_rand);
            let ray = self.camera.get_ray(rng, u, v);
            // Start the primary here from here
            color_accum += ray_color(rng, ray, &self.scene, self.max_depth, ray_count);
        }
        color_accum /= self.samples_per_pixel as f32;

        color_accum
    }

    pub fn render_frame(&self, channel_send: Sender<BufferPacket>) {
        println!("Start render");
        let time_start = std::time::Instant::now();
        let atomic_ray_count = AtomicU64::new(0);
        let atomic_line = AtomicU32::new(0);

        (0..self.image_height).into_par_iter().for_each(|_| {
            let line = atomic_line.fetch_add(1, Ordering::Relaxed);
            let mut packet = BufferPacket {
                pixels: Vec::with_capacity(self.image_width as usize),
            };
            let mut rng = RayRng::new(line as u64);
            for x in 0..self.image_width as u32 {
                let mut ray_count: u32 = 0;
                let col = self.render_pixel(x, line, &mut rng, &mut ray_count);
                atomic_ray_count.fetch_add(ray_count as u64, Ordering::Relaxed);
                packet
                    .pixels
                    .push((x, line, color_display_from_render(col)));
            }
            channel_send.send(packet).unwrap();
        });

        let time_elapsed = time_start.elapsed();
        let ray_count = atomic_ray_count.load(Ordering::Acquire);
        let ray_count_f32 = ray_count as f32;
        let mrays_sec = (ray_count_f32 / time_elapsed.as_secs_f32()) / 1000000.0;

        println!("Stop render");
        println!(
            "Time: {0}ms MRays/sec {1:.3}",
            time_elapsed.as_millis(),
            mrays_sec
        );

        drop(channel_send);
    }
}
