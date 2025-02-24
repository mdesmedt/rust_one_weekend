mod camera;
mod material;
mod object;
mod render;
mod scene;
mod shared;

use std::thread;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use camera::*;
use material::*;
use object::*;
use scene::*;
use shared::*;

use rayon::prelude::*;
use crossbeam_channel::unbounded;
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;
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
    width: usize,
    height: usize,
}

impl RenderBuffer {
    pub fn new(width: usize, height: usize, samples_per_pixel: u32) -> RenderBuffer {
        let buffer_display: Vec<ColorDisplay> = vec![0; width * height];

        RenderBuffer {
            buffer_display,
            width,
            height,
        }
    }
}

struct BufferPacket {
    pixels: Vec<(usize, usize, ColorDisplay)>
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

    // Limit to max ~10 fps update rate
    window.set_target_fps(30);

    // Create render buffer which holds all useful structs for rendering
    let mut render_buffer = RenderBuffer::new(WIDTH, HEIGHT, SAMPLES_PER_PIXEL);

    // Create teh scene
    let mut scene = one_weekend_scene();

    // Build the BVH
    scene.build_bvh();

    // Create the renderer
    let aspect_ratio = (WIDTH as f32) / (HEIGHT as f32);

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
        render::Renderer::new(WIDTH as u32, HEIGHT as u32, SAMPLES_PER_PIXEL, scene, cam);

    // Create channels
    let (channel_send, channel_receive) = unbounded();

    // Kick off renderer
    thread::spawn(move || {
        for line in 0..HEIGHT {
            let mut packet= BufferPacket {
                pixels: Vec::new()
            };
            let start = line * WIDTH;
            let end = start + WIDTH;
            (start..end)
                .into_par_iter()
                .map(|index| {
                    let x = index % WIDTH;
                    let y = index / WIDTH;
                    let col = render_worker.render_pixel(x as u32, y as u32);
                    (x, y, color_display_from_render(col))
                }).collect_into_vec(&mut packet.pixels);
            channel_send.send(packet).unwrap();
        }
    });

    // Window loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        {
            for packet in channel_receive.try_iter() {
                for pixel in packet.pixels {
                    let index = pixel.0 + pixel.1 * WIDTH;
                    render_buffer.buffer_display[index] = pixel.2;
                }
            }
            window
            .update_with_buffer(&render_buffer.buffer_display, WIDTH, HEIGHT)
            .unwrap();
        }
    }

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

/*
#[cfg(test)]
mod tests {
    use super::*;

    /// Tests if two renders produce exactly the same image
    #[test]
    fn test_determnism() {
        let mut render_buffer_a = RenderBuffer::new(100, 100, 8);
        render_buffer_a.render_worker.render_frame();
        render_buffer_a.update_buffer();
        let mut render_buffer_b = RenderBuffer::new(100, 100, 8);
        render_buffer_b.render_worker.render_frame();
        render_buffer_b.update_buffer();
        for i in 0..render_buffer_a.buffer_display.len() {
            let a = render_buffer_a.buffer_display[i];
            let b = render_buffer_a.buffer_display[i];
            assert_eq!(a, b);
        }
    }
}
*/
