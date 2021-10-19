use scoped_threadpool::Pool;
use spiral::ChebyshevIterator;

use crossbeam::atomic::AtomicCell;
use crossbeam_channel::bounded;

use crate::camera::*;
use crate::scene::*;
use crate::shared::*;

/// Coordinates for a block to render
pub struct RenderBlock {
    pub block_index: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Color>
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

        let width = x_end - x;
        let height = y_end - y;

        let mut rb = RenderBlock {
            block_index: self.block_index,
            x: x,
            y: y,
            width: width,
            height: height,
            pixels: Vec::new(),
        };

        // Allocate exactly enough space for pixels in the renderblock, to avoid allocation later in the renderer
        rb.pixels.reserve_exact((width * height) as usize);

        self.block_index += 1;

        return Some(rb);
    }
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

/// Renderer which generates pixels using the scene and camera, and sends them back using a crossbeam channel
pub struct Renderer {
    image_width: u32,
    image_height: u32,
    channel_sender: crossbeam_channel::Sender<RenderBlock>,
    channel_receiver: crossbeam_channel::Receiver<RenderBlock>,
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
        let mut blocks: Vec<RenderBlock> = blocker.collect();

        // Set up ChebyshevIterator. A bit awkward because it is square and generates out of bound XY which we need to check.
        let radius = ((std::cmp::max(block_count_x, block_count_y) / 2) + 1) as u16;
        let center_x = block_count_x / 2 - 1;
        let center_y = block_count_y / 2 - 1;

        let mut spiral_indices = Vec::new();
        // Loop blocks in spiral order using ChebyshevIterator
        for (block_x, block_y) in ChebyshevIterator::new(center_x, center_y, radius) {
            if block_x < 0 || block_x >= block_count_x || block_y < 0 || block_y >= block_count_y {
                continue; // Block out of bounds, ignore.
            }
            let block_index = (block_y * block_count_x + block_x) as usize;
            spiral_indices.push(block_index as u32);
        }

        blocks.sort_by_key(|rb| spiral_indices.iter().position(|&i| i == rb.block_index));

        let ref atomic_ray_count = AtomicCell::new(0u64);

        let mut threadpool = Pool::new(num_cpus::get() as u32);

        threadpool.scoped(|scoped| {
            // Loop blocks in the image blocker and spawn renderblock tasks
            for mut renderblock in blocks {
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
                            for _ in 0..self.samples_per_pixel {
                                let u = u_base + rng.gen_range(0.0..u_rand);
                                let v = v_base + rng.gen_range(0.0..v_rand);
                                let ray = self.camera.get_ray(u, v);
                                // Start the primary here from here
                                color_accum +=
                                    ray_color(ray, &self.scene, self.max_depth, &mut ray_count);
                            }
                            color_accum /= self.samples_per_pixel as f32;

                            renderblock.pixels.push(color_accum);
                        }); // for_each pixel

                        if self.keep_rendering.load() {
                            self.channel_sender.send(renderblock).unwrap();
                        }
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
    pub fn poll_results(&self) -> Vec<RenderBlock> {
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
