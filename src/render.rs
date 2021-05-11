use scoped_threadpool::Pool;
use spiral::ChebyshevIterator;

use crossbeam::atomic::AtomicCell;
use crossbeam_channel::bounded;

use crate::camera::*;
use crate::scene::*;
use crate::shared::*;

/// Coordinates for a block to render
#[derive(Copy, Clone)]
pub struct RenderBlock {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Generates blocks of up to width,height for an image of width,height
pub struct ImageBlocker {
    pub image_width: u32,
    pub image_height: u32,
    pub block_width: u32,
    pub block_height: u32,
    pub block_count_x: u32,
    pub block_count_y: u32,
    block_index: u32,
}

impl ImageBlocker {
    fn new(image_width: u32, image_height: u32) -> Self {
        let block_width = 32;
        let block_height = 32;
        ImageBlocker {
            image_width: image_width,
            image_height: image_height,
            block_width: block_width,
            block_height: block_height,
            block_count_x: ceil_div(image_width, block_width),
            block_count_y: ceil_div(image_height, block_height),
            block_index: 0,
        }
    }
}

/// Iterator which generates a series of RenderBlock for the image
impl Iterator for ImageBlocker {
    type Item = RenderBlock;

    fn next(&mut self) -> Option<RenderBlock> {
        let block_count = self.block_count_x * self.block_count_y;

        if self.block_index >= block_count {
            return None;
        }

        let block_x = self.block_index % self.block_count_x;
        let block_y = self.block_index / self.block_count_x;

        let x = block_x * self.block_width;
        let y = block_y * self.block_width;
        let x_end = std::cmp::min((block_x + 1) * self.block_width, self.image_width);
        let y_end = std::cmp::min((block_y + 1) * self.block_height, self.image_height);

        let rb = RenderBlock {
            x: x,
            y: y,
            width: x_end - x,
            height: y_end - y,
        };

        self.block_index += 1;

        return Some(rb);
    }
}

/// A fully rendered pixel
pub struct PixelResult {
    pub x: u32,
    pub y: u32,
    pub color: Color,
}

/// Recursive ray tracing
fn ray_color(ray: Ray, scene: &Scene, depth: i32, ray_count: &mut u32) -> Color {
    if depth <= 0 {
        return Color::ZERO;
    }

    // Intersect scene
    let query = RayQuery {
        ray: ray,
        t_min: TRACE_EPSILON,
        t_max: TRACE_INFINITY,
    };
    let hit_option = scene.intersect(query);
    *ray_count += 1;

    // If we hit something
    if let Some(hit) = hit_option {
        let scatter_option = hit.material.scatter(&ray, &hit);

        // Recurse
        if let Some(scatter) = scatter_option {
            return scatter.attenuation
                * ray_color(scatter.scattered_ray, scene, depth - 1, ray_count);
        }

        return Color::ZERO;
    }

    // Background
    let unit_direction = ray.direction.normalize();
    let t = 0.5 * (unit_direction.y + 1.0);
    return (1.0 - t) * Color::new(1.0, 1.0, 1.0) + t * Color::new(0.5, 0.7, 1.0);
}

/// Recursive ray tracing
fn render_packet(
    packet: &mut RayPacket,
    scene: &Scene,
    depth: i32,
    ray_count: &mut u32,
) -> [Color; TRACE_PACKET_SIZE] {
    if depth <= 0 {
        return [Color::ZERO; TRACE_PACKET_SIZE];
    }

    *ray_count += packet.ray_live_count as u32;

    let mut color_results = [Color::ZERO; TRACE_PACKET_SIZE];
    let mut attenuations: [Color; TRACE_PACKET_SIZE] = [Color::ONE; TRACE_PACKET_SIZE];
    let intersect_results = scene.intersect_packet(packet);
    for i in 0..TRACE_PACKET_SIZE {
        if packet.is_ray_live[i] {
            let ray = packet.rays[i];
            let hit_option = &intersect_results[i];
            if let Some(hit) = hit_option {
                let scatter_option = hit.material.scatter(&ray, &hit);
                if let Some(scatter) = scatter_option {
                    attenuations[i] = scatter.attenuation;
                    packet.update_ray(i, scatter.scattered_ray);
                } else {
                    packet.end_ray(i);
                }
            } else {
                // Background
                let unit_direction = ray.direction.normalize();
                let t = 0.5 * (unit_direction.y + 1.0);
                color_results[i] =
                    (1.0 - t) * Color::new(1.0, 1.0, 1.0) + t * Color::new(0.5, 0.7, 1.0);
                packet.end_ray(i);
            }
        }
    }

    if packet.ray_live_count > 0 {
        let color_recursed = render_packet(packet, scene, depth - 1, ray_count);
        for i in 0..TRACE_PACKET_SIZE {
            color_results[i] += color_recursed[i] * attenuations[i];
        }
    }

    return color_results;
}

/// Renderer which generates pixels using the scene and camera, and sends them back using a crossbeam channel
pub struct Renderer {
    image_width: u32,
    image_height: u32,
    channel_sender: crossbeam_channel::Sender<PixelResult>,
    channel_receiver: crossbeam_channel::Receiver<PixelResult>,
    keep_rendering: AtomicCell<bool>,
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
        let (s, r) = bounded(image_width as usize * image_height as usize);
        Renderer {
            image_width: image_width,
            image_height: image_height,
            channel_sender: s,
            channel_receiver: r,
            keep_rendering: AtomicCell::new(true),
            scene: scene,
            camera: camera,
            samples_per_pixel: samples_per_pixel,
            max_depth: 50,
        }
    }

    pub fn render_frame(&self) {
        println!("Start render");
        let time_start = std::time::Instant::now();

        // Generate blocks to render the image
        let blocker = ImageBlocker::new(self.image_width, self.image_height);
        let block_count_x = blocker.block_count_x as i32;
        let block_count_y = blocker.block_count_y as i32;
        let blocks: Vec<RenderBlock> = blocker.collect();

        // Set up ChebyshevIterator. A bit awkward because it is square and generates out of bound XY which we need to check.
        let radius = ((std::cmp::max(block_count_x, block_count_y) / 2) + 1) as u16;
        let center_x = block_count_x / 2 - 1;
        let center_y = block_count_y / 2 - 1;
        let mut spiral_blocks = Vec::new();

        // Loop blocks in spiral order using ChebyshevIterator
        for (block_x, block_y) in ChebyshevIterator::new(center_x, center_y, radius) {
            if block_x < 0 || block_x >= block_count_x || block_y < 0 || block_y >= block_count_y {
                continue; // Block out of bounds, ignore.
            }
            let block_index = (block_y * block_count_x + block_x) as usize;
            spiral_blocks.push(blocks[block_index])
        }

        let ref atomic_ray_count = AtomicCell::new(0u64);

        let mut threadpool = Pool::new(num_cpus::get() as u32);

        threadpool.scoped(|scoped| {
            // Loop blocks in the image blocker and spawn renderblock tasks
            for renderblock in spiral_blocks {
                scoped.execute(move || {
                    // Begin of thread
                    let num_pixels = renderblock.width * renderblock.height;
                    let mut ray_count = 0;
                    let mut rng = rand::thread_rng();
                    if self.keep_rendering.load() {
                        (0..num_pixels).into_iter().for_each(|index| {
                            // Compute pixel location
                            let x = renderblock.x + index % renderblock.width;
                            let y =
                                renderblock.y + (index / renderblock.width) % renderblock.height;

                            // Set up supersampling
                            let mut color_accum = Color::ZERO;
                            let u_base = x as f32 / (self.image_width as f32 - 1.0);
                            let v_base = (self.image_height - y - 1) as f32
                                / (self.image_height as f32 - 1.0);
                            let u_rand = 1.0 / (self.image_width as f32 - 1.0);
                            let v_rand = 1.0 / (self.image_height as f32 - 1.0);

                            // Supersample this pixel
                            if TRACE_PACKET {
                                for _ in 0..(self.samples_per_pixel / TRACE_PACKET_SIZE as u32) {
                                    let rays = <[Ray; TRACE_PACKET_SIZE]>::init_with(|| {
                                        let u = u_base + rng.gen_range(0.0..u_rand);
                                        let v = v_base + rng.gen_range(0.0..v_rand);
                                        self.camera.get_ray(u, v)
                                    });

                                    let mut packet = RayPacket::new(rays);
                                    // Start the primary here from here
                                    let color_results = render_packet(
                                        &mut packet,
                                        &self.scene,
                                        self.max_depth,
                                        &mut ray_count,
                                    );
                                    for c in std::array::IntoIter::new(color_results) {
                                        color_accum += c;
                                    }
                                }
                            } else {
                                for _ in 0..self.samples_per_pixel {
                                    let u = u_base + rng.gen_range(0.0..u_rand);
                                    let v = v_base + rng.gen_range(0.0..v_rand);
                                    let ray = self.camera.get_ray(u, v);
                                    // Start the primary here from here
                                    color_accum +=
                                        ray_color(ray, &self.scene, self.max_depth, &mut ray_count);
                                }
                            }
                            color_accum /= self.samples_per_pixel as f32;

                            // Send the result back
                            let result = PixelResult {
                                x: x,
                                y: y,
                                color: color_accum,
                            };
                            if self.keep_rendering.load() {
                                self.channel_sender.send(result).unwrap();
                            }
                        }); // for_each pixel
                    } // check keep_rendering
                    atomic_ray_count.fetch_add(ray_count as u64);
                    // End of thread
                }); // execute
            } // loop blocker
        }); // scoped

        let time_elapsed = time_start.elapsed();
        let ray_count = atomic_ray_count.load();
        let ray_count_f32 = ray_count as f32;
        let mrays_sec = (ray_count_f32 / time_elapsed.as_secs_f32()) / 1000000.0;

        println!("Stop render");
        println!(
            "Time: {0}ms MRays/sec {1:.3}",
            time_elapsed.as_millis(),
            mrays_sec
        );
    }

    /// Returns fully rendered pixels in the channel
    pub fn poll_results(&self) -> Vec<PixelResult> {
        let mut results = Vec::new();
        let mut limit = self.image_width * self.image_height;
        while !self.channel_receiver.is_empty() {
            let res = self.channel_receiver.recv().unwrap();
            results.push(res);
            limit -= 1;
            if limit == 0 {
                break;
            }
        }
        results
    }

    /// Request a currently ongoing render to stop looping
    pub fn stop_render(&self) {
        // First flag boolean
        self.keep_rendering.store(false);
        // Then drain channel
        while !self.channel_receiver.is_empty() {
            let _ = self.channel_receiver.recv();
        }
    }
}
