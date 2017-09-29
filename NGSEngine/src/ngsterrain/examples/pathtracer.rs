//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngsterrain;
extern crate sdl2;
extern crate clap;
extern crate rand;
extern crate xdispatch;

use std::fs::File;
use std::io::BufReader;
use std::time::Instant;
use rand::Rng;
use ngsterrain::cgmath;
use self::cgmath::{Vector3, Vector2, vec3, vec2};
use self::cgmath::prelude::*;

#[derive(Debug)]
struct State {
    eye: Vector3<f32>,
    velocity: Vector3<f32>,
    angle: Vector3<f32>,
}

impl State {
    fn new() -> State {
        State {
            eye: vec3(256.0, 256.0, 64.0),
            velocity: Vector3::zero(),
            angle: vec3(-0.1, 0.0, 0.0),
        }
    }

    fn handle_event(&mut self, e: &sdl2::event::Event) {
        use sdl2::event::Event;
        match e {
            &Event::MouseMotion { xrel, yrel, .. } => {
                self.angle.x += yrel as f32 * 0.001;
                self.angle.z += xrel as f32 * 0.001;
            }
            _ => {}
        }
    }

    fn update(&mut self, dt: f32, _sdl: &sdl2::Sdl, event_pump: &sdl2::EventPump) {
        self.eye += self.velocity * dt;

        let rp = self.render_params();
        self.velocity *= 0.1f32.powf(dt);

        use sdl2::keyboard::Scancode;
        let state = sdl2::keyboard::KeyboardState::new(event_pump);
        let speed = if state.is_scancode_pressed(Scancode::LShift) ||
            state.is_scancode_pressed(Scancode::RShift)
        {
            48.0
        } else {
            16.0
        };
        if state.is_scancode_pressed(Scancode::A) {
            self.velocity -= rp.axis[0] * (dt * speed);
        } else if state.is_scancode_pressed(Scancode::D) {
            self.velocity += rp.axis[0] * (dt * speed);
        }
        if state.is_scancode_pressed(Scancode::W) {
            self.velocity += rp.axis[2] * (dt * speed);
        } else if state.is_scancode_pressed(Scancode::S) {
            self.velocity -= rp.axis[2] * (dt * speed);
        }
    }

    fn render_params(&self) -> RenderParams {
        use cgmath::{Basis3, Rad};
        let basis = Basis3::from_angle_x(Rad(std::f32::consts::FRAC_PI_2)) *
            Basis3::from_angle_y(Rad(self.angle.z)) *
            Basis3::from_angle_x(Rad(self.angle.x)) *
            Basis3::from_angle_z(Rad(self.angle.y));
        let mat = basis.as_ref();
        RenderParams {
            eye: self.eye,
            axis: [
                mat * Vector3::unit_x(),
                mat * Vector3::unit_y(),
                mat * Vector3::unit_z(),
            ],
            fov: 0.8,
        }
    }
}

#[derive(Debug, Clone)]
struct Sampler {
    rng: rand::XorShiftRng,
}

impl Sampler {
    fn new() -> Self {
        Self { rng: rand::XorShiftRng::new_unseeded() }
    }

    fn gaussian(&mut self) -> f32 {
        (self.rng.next_f32() + self.rng.next_f32() + self.rng.next_f32() + self.rng.next_f32() -
            2.0)
    }

    fn reconstruction_filter(&mut self) -> Vector2<f32> {
        Vector2::new(self.gaussian(), self.gaussian())
    }

    fn sample_diffuse(&mut self) -> Vector3<f32> {
        loop {
            let x = self.rng.next_f32() * 2.0 - 1.0;
            let y = self.rng.next_f32() * 2.0 - 1.0;
            let ln = x.mul_add(x, y * y);
            if ln < 1.0 {
                let z = (1.0 - ln).sqrt();
                break vec3(x, y, z);
            }
        }
    }
}

#[derive(Debug)]
struct PerfCounter {
    last_measure: Instant,
    count: f64,
    last_rate: f64,
}

impl PerfCounter {
    fn new() -> Self {
        Self {
            last_measure: Instant::now(),
            count: 0.0,
            last_rate: 0.0,
        }
    }

    fn log(&mut self, value: f64) {
        self.count += value;

        let dt = self.last_measure.elapsed();
        let dt = dt.subsec_nanos() as f64 * 1.0e-9 + dt.as_secs() as f64;
        if dt >= 0.2 {
            self.last_rate = self.count / dt;
            self.count = 0.0;
            self.last_measure = Instant::now();
        }
    }

    fn rate(&self) -> f64 {
        self.last_rate
    }
}

#[derive(Debug)]
struct RenderParams {
    eye: Vector3<f32>,
    axis: [Vector3<f32>; 3],
    fov: f32,
}

impl RenderParams {
    fn primary_ray(&self, mut v: Vector2<f32>) -> [Vector3<f32>; 3] {
        v *= self.fov;
        [
            self.axis[2] + self.axis[0] * v.x + self.axis[1] * v.y,
            self.axis[0] * self.fov,
            self.axis[1] * self.fov,
        ]
    }
}

#[derive(Debug)]
struct Renderer {
    terrain: ngsterrain::Terrain,
    sampler: Sampler,
    sun_dir: Vector3<f32>,

    samples_counter: PerfCounter,
    fps_counter: PerfCounter,
}

#[derive(Debug)]
enum Material {
    Object {
        position: Vector3<f32>,
        normal: Vector3<f32>,
        tangent: Vector3<f32>,
        binormal: Vector3<f32>,
        albedo: Vector3<f32>,
    },
    Sky(Vector3<f32>),
}

impl Renderer {
    fn new(terrain: ngsterrain::Terrain) -> Self {
        Self {
            terrain,
            sampler: Sampler::new(),
            sun_dir: vec3(1.0, 0.5, 1.0).normalize(),

            samples_counter: PerfCounter::new(),
            fps_counter: PerfCounter::new(),
        }
    }

    fn raytrace(&self, start: Vector3<f32>, dir: Vector3<f32>, max_dist: f32) -> Material {
        use ngsterrain::{raytrace, SolidVoxel, ColoredVoxel};

        match raytrace::raytrace(&self.terrain, start, start + dir * max_dist) {
            raytrace::RaytraceResult::Hit(hit) => {
                let voxel = self.terrain
                    .get_voxel(hit.voxel)
                    .and_then(|sv| match sv {
                        SolidVoxel::Colored(colored) => Some(colored.into_owned()),
                        SolidVoxel::Uncolored => None,
                    })
                    .unwrap_or_else(|| ColoredVoxel::default(hit.voxel));
                let color = voxel.color();

                let normal = hit.normal.as_vector3();
                let tangent = vec3(normal.z, normal.x, normal.y);
                let binormal = vec3(tangent.z, tangent.x, tangent.y);

                let albedo = vec3(color[0], color[1], color[2]).cast() * (1.0 / 255.0);

                // 2.0 gamma
                let albedo = albedo.mul_element_wise(albedo);

                Material::Object {
                    position: hit.position,
                    normal,
                    tangent,
                    binormal,
                    albedo,
                }
            }
            _ => Material::Sky(
                vec3(0.8 + dir.x * 0.3, 1.0, 1.0 - dir.x * 0.7) +
                    vec3(1.0, 0.9, 0.8) * (500.0 * (dir.dot(self.sun_dir) - 0.95).max(0.0)),
            ),
        }
    }

    fn pathtrace(
        &self,
        mut start: Vector3<f32>,
        mut dir: Vector3<f32>,
        sampler: &mut Sampler,
    ) -> Vector3<f32> {
        let mut coef: Vector3<f32> = vec3(1.0, 1.0, 1.0);

        let mut dist = 128.0;

        for _ in 0..3 {
            let mat = self.raytrace(start, dir, dist);
            match mat {
                Material::Object {
                    position,
                    normal,
                    tangent,
                    binormal,
                    albedo,
                } => {
                    coef.mul_assign_element_wise(albedo);

                    let r = sampler.sample_diffuse();
                    start = position + normal * 0.001;
                    dir = normal * r.z + tangent * r.x + binormal * r.y;
                }
                Material::Sky(color) => {
                    return color.mul_element_wise(coef);
                }
            }

            // Limit the clip distance of secondary rays (for performance)
            dist = 32.0;
        }

        Vector3::zero()
    }

    fn render_to(&mut self, surf: &mut sdl2::surface::SurfaceRef, params: &RenderParams) {
        assert_eq!(
            surf.pixel_format_enum(),
            sdl2::pixels::PixelFormatEnum::ARGB8888
        );
        let ((width, height), pitch) = (surf.size(), surf.pitch());
        let pixels = surf.without_lock_mut().unwrap();

        const UNDERSAMPLE: u32 = 8;
        const SAMPLES_PER_PIXEL: u32 = 32;

        assert!(width % UNDERSAMPLE == 0 && height % UNDERSAMPLE == 0);

        // temporal dithering
        for _ in 0..4 {
            self.sampler.rng.next_u32();
        }

        struct SendPtr<T>(*mut T);

        unsafe impl<T> Sync for SendPtr<T> {}
        unsafe impl<T> Send for SendPtr<T> {}

        let pixels_p = SendPtr(pixels.as_mut_ptr());

        let factor = 2.0 / width as f32;

        xdispatch::Queue::global(xdispatch::QueuePriority::Default)
            .apply((height / UNDERSAMPLE) as usize, |y| {
                let y = y as u32;
                let mut sampler = self.sampler.clone();

                use rand::SeedableRng;
                let x = sampler.rng.next_u32();
                sampler.rng.reseed([x ^ y, x ^ y ^ 1, x ^ y ^ 2, x ^ y ^ 4]);

                for x in 0..width / UNDERSAMPLE {
                    let centered_pos = vec2(x * UNDERSAMPLE, y * UNDERSAMPLE).cast::<i32>() -
                        vec2(width / 2, height / 2).cast::<i32>();
                    let mut norm_pos = centered_pos.cast::<f32>() * factor;
                    norm_pos.y = -norm_pos.y;
                    let dir = params.primary_ray(norm_pos);

                    let mut color = Vector3::zero();

                    for _ in 0..SAMPLES_PER_PIXEL {
                        let mut aa_dir = dir[0];

                        // Apply reconstruction filter
                        let rf = sampler.reconstruction_filter() * (factor * UNDERSAMPLE as f32);
                        aa_dir += dir[1] * rf.x;
                        aa_dir += dir[2] * rf.y;

                        // Perform a path tracing
                        color += self.pathtrace(params.eye, aa_dir.normalize(), &mut sampler);
                    }

                    color *= 1.0 / SAMPLES_PER_PIXEL as f32;

                    fn aces_film(x: f32) -> f32 {
                        let (a, b, c, d, e) = (2.51, 0.03, 2.43, 0.59, 0.14);
                        ((x * (a * x + b)) / (x * (c * x + d) + e)).min(1.0)
                    }

                    // Apply ACES tone curve & gamma correction
                    color.x = aces_film(color.x).sqrt();
                    color.y = aces_film(color.y).sqrt();
                    color.z = aces_film(color.z).sqrt();
                    color *= 255.0;

                    let color_i: Vector3<u32> = color.cast();

                    let color_raw = 0xff000000 | color_i.x | (color_i.y << 8) | (color_i.z << 16);

                    for sy in 0..UNDERSAMPLE {
                        for sx in 0..UNDERSAMPLE {
                            unsafe {
                                *(pixels_p.0.offset(
                                    ((x * UNDERSAMPLE + sx) * 4 + (y * UNDERSAMPLE + sy) * pitch) as
                                        isize,
                                ) as *mut u32) = color_raw
                            };
                        }
                    }
                }
            });

        self.samples_counter.log(
            (width * height / UNDERSAMPLE / UNDERSAMPLE * SAMPLES_PER_PIXEL) as f64,
        );
        self.fps_counter.log(1.0);
    }
}

fn main() {
    use clap::{App, Arg};
    // Use `clap` to parse command-line arguments
    let matches = App::new("pathtracer")
        .author("yvt <i@yvt.jp>")
        .about(
            "interractive viewer for voxel terrain data using the path tracing algorithm",
        )
        .arg(
            Arg::with_name("INPUT")
                .help("file to display; the Voxlap VXL format is supported")
                .required(true)
                .index(1),
        )
        .get_matches();

    // Load the input vox file
    println!("Loading the input file");
    let input_path = matches.value_of_os("INPUT").unwrap();
    let file = File::open(input_path).unwrap();
    let mut reader = BufReader::new(file);
    let terrain = ngsterrain::io::from_voxlap_vxl(vec3(512, 512, 64), &mut reader).unwrap();
    terrain.validate().unwrap();
    let mut renderer = Renderer::new(terrain);

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video = sdl_context.video().unwrap();

    let mut window = video
        .window("pathtracer", 720, 480)
        .position_centered()
        .build()
        .unwrap();

    let mut surface = {
        let window_surface = window.surface(&event_pump).unwrap();
        let surf = sdl2::surface::Surface::new(
            window_surface.width(),
            window_surface.height(),
            sdl2::pixels::PixelFormatEnum::ARGB8888,
        );
        surf.unwrap()
    };

    let mut state = State::new();

    use sdl2::keyboard::Keycode;
    use sdl2::event::Event;

    let mut last_time = Instant::now();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'running,
                Event::MouseButtonDown { .. } => {
                    sdl_context.mouse().show_cursor(false);
                    sdl_context.mouse().set_relative_mouse_mode(true);
                }
                e => {
                    state.handle_event(&e);
                }
            }
        }

        let dur = last_time.elapsed();
        last_time = Instant::now();

        let dur = dur.subsec_nanos() as f32 * 1.0e-9;

        state.update(dur, &sdl_context, &event_pump);

        renderer.render_to(&mut surface, &state.render_params());

        {
            let mut window_surface = window.surface(&event_pump).unwrap();
            surface.blit(None, &mut window_surface, None).unwrap();
            window_surface.update_window().unwrap();
        }

        let title =
            format!(
            "pathtracer [{:.2} sps, {:.2} fps]",
            renderer.samples_counter.rate(),
            renderer.fps_counter.rate(),
        );
        window.set_title(&title).unwrap();
    }
}
