//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngsterrain;
extern crate sdl2;
extern crate clap;
extern crate rand;

use std::fs::File;
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
        if state.is_scancode_pressed(Scancode::A) {
            self.velocity -= rp.axis[0] * (dt * 16.0);
        } else if state.is_scancode_pressed(Scancode::D) {
            self.velocity += rp.axis[0] * (dt * 16.0);
        }
        if state.is_scancode_pressed(Scancode::W) {
            self.velocity += rp.axis[2] * (dt * 16.0);
        } else if state.is_scancode_pressed(Scancode::S) {
            self.velocity -= rp.axis[2] * (dt * 16.0);
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
            fov: 0.5,
        }
    }
}

#[derive(Debug)]
struct Sampler {
    rng: rand::XorShiftRng,
}

impl Sampler {
    fn new() -> Self {
        Self { rng: rand::XorShiftRng::new_unseeded() }
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
struct RenderParams {
    eye: Vector3<f32>,
    axis: [Vector3<f32>; 3],
    fov: f32,
}

impl RenderParams {
    fn primary_ray(&self, mut v: Vector2<f32>) -> Vector3<f32> {
        v *= self.fov;
        self.axis[2] + self.axis[0] * v.x + self.axis[1] * v.y
    }
}

#[derive(Debug)]
struct Renderer {
    terrain: ngsterrain::Terrain,
    sampler: Sampler,
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

                let albedo = vec3(color[0], color[1], color[2]).cast() * (0.8 / 255.0);

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
            _ => Material::Sky(vec3(1.0, 1.0, 1.0)),
        }
    }

    fn pathtrace(&mut self, mut start: Vector3<f32>, mut dir: Vector3<f32>) -> Vector3<f32> {
        let mut coef: Vector3<f32> = vec3(1.0, 1.0, 1.0);

        let mut dist = 64.0;

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

                    let r = self.sampler.sample_diffuse();
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

        for y in 0..height {
            for x in 0..width {
                let centered_pos = vec2(x, y).cast::<i32>() -
                    vec2(width / 2, height / 2).cast::<i32>();
                let mut norm_pos = centered_pos.cast::<f32>() * (2.0 / width as f32);
                norm_pos.y = -norm_pos.y;
                let dir = params.primary_ray(norm_pos).normalize();

                let mut color = self.pathtrace(params.eye, dir);

                color.x = color.x.max(0.0).min(1.0).sqrt();
                color.y = color.y.max(0.0).min(1.0).sqrt();
                color.z = color.z.max(0.0).min(1.0).sqrt();
                color *= 255.0;

                let color_i: Vector3<u32> = color.cast();

                let pixel = unsafe { pixels.as_mut_ptr().offset((x * 4 + y * pitch) as isize) } as
                    *mut u32;
                unsafe {
                    *pixel = 0xff000000 | color_i.x | (color_i.y << 8) | (color_i.z << 16);
                };
            }
        }
    }
}

fn main() {
    use clap::{App, Arg};
    // Use `clap` to parse command-line arguments
    let matches = App::new("pathtracer")
        .author("yvt <i@yvt.jp>")
        .about("interractive pathtracer for voxel terrain data")
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
    let mut file = File::open(input_path).unwrap();
    let terrain = ngsterrain::io::from_voxlap_vxl(Vector3::new(512, 512, 64), &mut file).unwrap();
    terrain.validate().unwrap();
    let mut renderer = Renderer::new(terrain);

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video = sdl_context.video().unwrap();

    let window = video
        .window("pathtracer", 360, 240)
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
    use std::time::Instant;

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

        let mut window_surface = window.surface(&event_pump).unwrap();
        surface.blit(None, &mut window_surface, None).unwrap();
        window_surface.update_window().unwrap();
    }
}
