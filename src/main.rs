mod camera;
mod material;
mod object;
mod render;
mod scene;
mod shared;

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use camera::*;
use material::*;
use object::*;
use scene::*;
use shared::*;

use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;
const SAMPLES_PER_PIXEL: u32 = 128;

/// Generate the ray tracing in one weekend scene
fn one_weekend_scene() -> Scene {
    let mut rng = RayRng::new(0);
    let mut scene = Scene::new();

    let mut spheres: Vec<(Point3, f32)> = Vec::new();
    let mut add_sphere =
        |spheres: &mut Vec<(Point3, f32)>, c: Point3, r: f32, mat: &Arc<dyn Material>| {
            scene.objects.push(Box::new(Sphere::new(c, r, mat)));
            spheres.push((c, r));
        };

    let sphere_intersects = |spheres: &Vec<(Point3, f32)>, c: Point3, r: f32| {
        spheres.iter().any(|s| (s.0 - c).length() < (s.1 + r))
    };

    let ground_material: Arc<dyn Material> = Arc::new(Lambertian {
        albedo: Color::new(0.5, 0.5, 0.5),
    });
    add_sphere(
        &mut spheres,
        Point3::new(0.0, -1000.0, -1.0),
        1000.0,
        &ground_material,
    );

    let material1: Arc<dyn Material> = Arc::new(Dielectric { ir: 1.5 });
    add_sphere(&mut spheres, Point3::new(0.0, 1.0, 0.0), 1.0, &material1);

    let material2: Arc<dyn Material> = Arc::new(Lambertian {
        albedo: Color::new(0.4, 0.2, 0.1),
    });
    add_sphere(&mut spheres, Point3::new(-4.0, 1.0, 0.0), 1.0, &material2);

    let material3: Arc<dyn Material> = Arc::new(Metal {
        albedo: Color::new(0.7, 0.6, 0.5),
        fuzz: 0.0,
    });
    add_sphere(&mut spheres, Point3::new(4.0, 1.0, 0.0), 1.0, &material3);

    for a in -11..11 {
        for b in -11..11 {
            let choose_mat = rng.gen_range(0.0..1.0);
            let mut center;
            // Find a position which doesn't intersect with any other sphere
            loop {
                center = Point3::new(
                    a as f32 + 0.9 * rng.gen_range(0.0..1.0),
                    0.2,
                    b as f32 + 0.9 * rng.gen_range(0.0..1.0),
                );
                if !sphere_intersects(&spheres, center, 0.2) {
                    break;
                }
            }

            if (center - Point3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                if choose_mat < 0.7 {
                    // diffuse
                    let albedo = color_random(&mut rng);
                    let sphere_material: Arc<dyn Material> = Arc::new(Lambertian { albedo });
                    add_sphere(&mut spheres, center, 0.2, &sphere_material);
                } else if choose_mat < 0.95 {
                    // metal
                    let albedo = color_random_range(&mut rng, 0.5..1.0);
                    let fuzz = rng.gen_range(0.0..0.5);
                    let sphere_material: Arc<dyn Material> = Arc::new(Metal { albedo, fuzz });
                    add_sphere(&mut spheres, center, 0.2, &sphere_material);
                } else {
                    // glass
                    let sphere_material: Arc<dyn Material> = Arc::new(Dielectric { ir: 1.5 });
                    add_sphere(&mut spheres, center, 0.2, &sphere_material);
                }
            }
        }
    }

    scene
}

struct RenderBuffer {
    buffer_display: Vec<ColorDisplay>,
    render_worker: Arc<render::Renderer>,
    width: usize,
    height: usize,
}

impl RenderBuffer {
    pub fn new(width: usize, height: usize, samples_per_pixel: u32) -> RenderBuffer {
        let buffer_display: Vec<ColorDisplay> = vec![0; width * height];

        let mut scene = one_weekend_scene();
        scene.build_bvh();

        let aspect_ratio = (width as f32) / (height as f32);

        let lookfrom = Point3::new(13.0, 2.0, 3.0);
        let lookat = Point3::new(0.0, 0.0, 0.0);
        let vup = Vec3::new(0.0, 1.0, 0.0);
        let dist_to_focus = 10.0;
        let aperture = 0.1;

        let cam = Camera::new(
            lookfrom,
            lookat,
            vup,
            20.0,
            aspect_ratio,
            aperture,
            dist_to_focus,
        );

        let render_worker =
            render::Renderer::new(width as u32, height as u32, samples_per_pixel, scene, cam);

        RenderBuffer {
            buffer_display,
            render_worker: Arc::new(render_worker),
            width,
            height,
        }
    }

    pub fn update_buffer(&mut self) -> bool {
        // Fetch rendered pixels
        let render_results = &self.render_worker.poll_results();
        let has_changed = !render_results.is_empty();
        for result in render_results {
            for i in 0..result.pixels.len() {
                let color = result.pixels[i];
                let x = result.x + (i as u32 % result.width);
                let y = result.y + (i as u32 / result.width);
                let index = index_from_xy(self.width as u32, self.height as u32, x, y);
                self.buffer_display[index] = color_display_from_render(color);
            }
        }
        has_changed
    }
}

fn main() {
    let mut window = Window::new(
        "Ray tracing in one weekend - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(100000)));

    // Create render buffer which holds all useful structs for rendering
    let mut render_buffer = RenderBuffer::new(WIDTH, HEIGHT, SAMPLES_PER_PIXEL);

    crossbeam::scope(|s| {
        // Start the render thread
        let render_worker = render_buffer.render_worker.clone();
        s.spawn(move |_| {
            render_worker.render_frame();
        });

        // Window loop
        while window.is_open() && !window.is_key_down(Key::Escape) {
            let has_changed = render_buffer.update_buffer();
            if has_changed {
                window
                    .update_with_buffer(&render_buffer.buffer_display, WIDTH, HEIGHT)
                    .unwrap();
            } else {
                window.update();
            }
        }

        render_buffer.render_worker.stop_render();
    })
    .unwrap();

    // If we get one argument, assume it's our output png filename
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let path = Path::new(&args[1]);
        let file = File::create(path).unwrap();
        let w = &mut BufWriter::new(file);

        // Write buffer_display as 8-bit RGB PNG
        let mut encoder = png::Encoder::new(w, WIDTH as u32, HEIGHT as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        let data: Vec<u8> = render_buffer
            .buffer_display
            .iter()
            .flat_map(|x| u8_vec_from_color_display(*x))
            .collect();
        writer.write_image_data(&data).unwrap();
    }
}
