//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(duration_float)]
use alt_fp::FloatOrdSet;
use cgmath::{prelude::*, vec2, vec3, vec4, Matrix3, Matrix4, Point2, Point3, Vector3, Vector4};
use glium::{backend::Facade, glutin, Surface};
use imgui::{im_str, ImGui};
use std::time::Instant;

use stygian;

mod lib {
    pub mod cube;
    pub mod depthvis;
    pub mod linedraw;
    #[path = "../../common/profmempool.rs"]
    pub mod profmempool;
    pub mod scene;
    #[path = "../../common/terrainload.rs"]
    pub mod terrainload;
    pub mod vxl2mesh;
}

fn main() {
    use clap::{App, Arg};
    // Use `clap` to parse command-line arguments
    let matches = App::new("stygian_demo1")
        .about("Stygian demo app 1")
        .arg(
            Arg::with_name("INPUT")
                .help("file to display; .vxl, .vox, and .glb.xz formats are supported")
                .index(1),
        )
        .get_matches();

    println!("Escape - Exit");
    println!("WASD - Move");
    println!("←↓↑→ - Look");

    // Load the input vox file
    println!("Loading the input file");
    let scene = if let Some(input_path) = matches.value_of_os("INPUT") {
        lib::scene::Scene::load(input_path)
    } else {
        lib::scene::Scene::load_derby_racers()
    };

    let mut state = State::new(&scene);
    let mut imgui = ImGui::init();
    let mut metrics = Metrics::new();

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new();
    let context = glutin::ContextBuilder::new()
        .with_depth_buffer(24)
        .with_vsync(true);
    let display = glium::Display::new(window, context, &events_loop).unwrap();
    let mut renderer = Renderer::new(&display, &mut imgui, &scene);
    let window = display.gl_window();

    use glutin::{ElementState, Event, VirtualKeyCode, WindowEvent};

    let mut last_time = Instant::now();

    let mut keep_running = true;

    while keep_running {
        events_loop.poll_events(|event| {
            imgui_winit_support::handle_event(
                &mut imgui,
                &event,
                window.get_hidpi_factor(),
                window.get_hidpi_factor(),
            );

            match event {
                Event::WindowEvent { event, .. } => {
                    state.handle_event(&event);

                    match event {
                        WindowEvent::CloseRequested => {
                            keep_running = false;
                        }
                        WindowEvent::KeyboardInput { input, .. } => match input.state {
                            ElementState::Pressed => match input.virtual_keycode {
                                Some(VirtualKeyCode::Escape) => {
                                    keep_running = false;
                                }
                                _ => (),
                            },
                            _ => (),
                        },
                        _ => (),
                    }
                }
                _ => (),
            }
        });

        let delta_time = last_time.elapsed();
        last_time = Instant::now();

        let delta_time = delta_time.subsec_nanos() as f32 * 1.0e-9;

        state.update(delta_time);
        let ui = state.ui(
            &mut imgui,
            imgui_winit_support::get_frame_size(&window, window.get_hidpi_factor()).unwrap(),
            &metrics,
            delta_time,
        );

        // drawing a frame
        let mut target = display.draw();
        renderer.render(&display, &state, ui, &mut metrics, &mut target);

        target.finish().unwrap();
    }
}

struct State {
    eye: Vector3<f32>,
    velocity: Vector3<f32>,
    angle: Vector3<f32>,
    angular_velocity: Vector3<f32>,
    keys: [bool; 16],
    show_opticast_samples: bool,
    show_depth: bool,
    show_birds_eye_view: bool,
}

impl State {
    fn new(scene: &lib::scene::Scene) -> State {
        State {
            eye: scene.camera_initial_position() - Point3::new(0.0, 0.0, 0.0),
            velocity: Vector3::zero(),
            angle: vec3(-0.4, 0.0, 0.0),
            angular_velocity: Vector3::zero(),
            keys: [false; 16],
            show_opticast_samples: false,
            show_depth: true,
            show_birds_eye_view: true,
        }
    }

    fn handle_event(&mut self, e: &glutin::WindowEvent) {
        if let glutin::WindowEvent::KeyboardInput { input, .. } = e {
            let pressed = input.state == glutin::ElementState::Pressed;
            let key = match input.virtual_keycode {
                Some(key) => key,
                None => return,
            };
            match key {
                glutin::VirtualKeyCode::Up => self.keys[0] = pressed,
                glutin::VirtualKeyCode::Down => self.keys[1] = pressed,
                glutin::VirtualKeyCode::Left => self.keys[2] = pressed,
                glutin::VirtualKeyCode::Right => self.keys[3] = pressed,
                glutin::VirtualKeyCode::A => self.keys[4] = pressed,
                glutin::VirtualKeyCode::D => self.keys[5] = pressed,
                glutin::VirtualKeyCode::W => self.keys[6] = pressed,
                glutin::VirtualKeyCode::S => self.keys[7] = pressed,
                glutin::VirtualKeyCode::LShift | glutin::VirtualKeyCode::RShift => {
                    self.keys[8] = pressed
                }
                _ => {}
            }
        }

        // FIXME: As it turned out, glutin didn't support relative mouse input
    }

    fn update(&mut self, dt: f32) {
        self.eye += self.velocity * dt;
        self.angle += self.angular_velocity * dt;
        self.angle.y = self.angular_velocity.z * 0.03;

        let view_mat = self.view_matrix();
        self.velocity *= 0.1f32.powf(dt);
        self.angular_velocity *= 0.1f32.powf(dt);

        let speed = if self.keys[8] { 48.0 } else { 16.0 };
        if self.keys[4] {
            self.velocity -= view_mat.transpose().x.truncate() * (dt * speed);
        } else if self.keys[5] {
            self.velocity += view_mat.transpose().x.truncate() * (dt * speed);
        }
        if self.keys[6] {
            self.velocity -= view_mat.transpose().z.truncate() * (dt * speed);
        } else if self.keys[7] {
            self.velocity += view_mat.transpose().z.truncate() * (dt * speed);
        }

        if self.keys[0] {
            self.angular_velocity.x -= 5.0 * dt;
        } else if self.keys[1] {
            self.angular_velocity.x += 5.0 * dt;
        }
        if self.keys[2] {
            self.angular_velocity.z += 5.0 * dt;
        } else if self.keys[3] {
            self.angular_velocity.z -= 5.0 * dt;
        }
    }

    fn ui<'ui, 'a: 'ui>(
        &mut self,
        imgui: &'a mut ImGui,
        frame_size: imgui::FrameSize,
        metrics: &Metrics,
        dt: f32,
    ) -> imgui::Ui<'ui> {
        let ui = imgui.frame(frame_size, dt);

        ui.window(im_str!("Options"))
            .always_auto_resize(true)
            .build(|| {
                ui.checkbox(
                    im_str!("Show opticast samples"),
                    &mut self.show_opticast_samples,
                );
                ui.checkbox(im_str!("Show depth"), &mut self.show_depth);
                ui.checkbox(
                    im_str!("Show birds'eye view"),
                    &mut self.show_birds_eye_view,
                );
            });

        ui.window(im_str!("Metrics"))
            .always_auto_resize(true)
            .build(|| {
                ui.text(format!("{:3.2} fps", metrics.fps_counter.rate()));

                ui.separator();
                if metrics.history_depth_build_time.len() > 0 {
                    let history = &metrics.history_depth_build_time;
                    ui.text(format!(
                        "Depth build time: {:5.2}us - {:5.2}us (avg: {:5.2}us)",
                        history.fmin() * 1.0e6,
                        history.fmax() * 1.0e6,
                        history.iter().fold(0.0, |x, &y| x + y) * 1.0e6 / history.len() as f32
                    ));
                    ui.plot_lines(im_str!(""), &history).build();
                }
                ui.text(format!(
                    "Objects: {:4}/{:4}",
                    metrics.num_rendered_objects, metrics.num_objects
                ));
                ui.plot_lines(im_str!(""), &metrics.history_num_rendered_objects)
                    .build();
            });

        ui
    }

    fn view_matrix(&self) -> Matrix4<f32> {
        use cgmath::{Basis3, Rad};
        let basis = Basis3::from_angle_x(Rad(std::f32::consts::FRAC_PI_2))
            * Basis3::from_angle_y(Rad(self.angle.z))
            * Basis3::from_angle_x(Rad(self.angle.x))
            * Basis3::from_angle_z(Rad(self.angle.y));
        let basis_mat: &Matrix3<f32> = basis.as_ref();

        Matrix4::from_cols(
            basis_mat.x.extend(0.0),
            basis_mat.y.extend(0.0),
            basis_mat.z.extend(0.0),
            vec4(0.0, 0.0, 0.0, 1.0),
        )
        .transpose()
            * Matrix4::from_translation(-self.eye)
    }

    fn render_params(&self, aspect: f32) -> RenderParams {
        use cgmath::{PerspectiveFov, Rad};

        let proj: Matrix4<f32> = PerspectiveFov {
            fovy: Rad(1.0),
            aspect,
            near: 0.5,
            far: 500.0,
        }
        .into();

        let view = self.view_matrix();

        RenderParams {
            camera_matrix: Matrix4::from_translation(vec3(0.0, 0.0, 0.5))
                * Matrix4::from_nonuniform_scale(1.0, 1.0, -0.5)
                * proj
                * view,
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
struct Metrics {
    fps_counter: PerfCounter,
    history_depth_build_time: Vec<f32>,
    history_num_rendered_objects: Vec<f32>,
    num_rendered_objects: usize,
    num_objects: usize,
}

impl Metrics {
    fn new() -> Self {
        Metrics {
            fps_counter: PerfCounter::new(),
            history_depth_build_time: Vec::new(),
            history_num_rendered_objects: Vec::new(),
            num_rendered_objects: 0,
            num_objects: 0,
        }
    }

    fn log_depth_build_time(&mut self, t: f32) {
        if self.history_depth_build_time.len() > 300 {
            self.history_depth_build_time.remove(0);
        }
        self.history_depth_build_time.push(t);
    }

    fn log_num_objects(&mut self, rendered: usize, total: usize) {
        if self.history_num_rendered_objects.len() > 300 {
            self.history_num_rendered_objects.remove(0);
        }
        self.history_num_rendered_objects.push(rendered as f32);
        self.num_rendered_objects = rendered;
        self.num_objects = total;
    }
}

#[derive(Debug)]
struct RenderParams {
    camera_matrix: Matrix4<f32>,
}

struct Renderer {
    imgui_renderer: imgui_glium_renderer::Renderer,

    scene_renderer: lib::scene::SceneRenderer,
    scene_instance: lib::scene::SceneInstance,

    sty_terrain: stygian::Terrain,
    sty_rast: stygian::TerrainRast,
    sty_depth: stygian::DepthImage,
    sty_model_matrix: Matrix4<f32>,

    linedraw: lib::linedraw::LineDraw,

    depthvis: lib::depthvis::DepthVis,

    birds_eye_view_matrix: Matrix4<f32>,
}

impl Renderer {
    fn new(facade: &impl Facade, imgui: &mut ImGui, scene: &lib::scene::Scene) -> Self {
        let imgui_renderer = imgui_glium_renderer::Renderer::init(imgui, facade).unwrap();

        let scene_renderer = lib::scene::SceneRenderer::new(facade);
        let scene_instance = scene_renderer.prepare_scene(facade, scene);

        println!("Initializing Stygian");
        let (sty_terrain, sty_model_matrix) = scene.make_sty_terrain();
        let sty_rast = stygian::TerrainRast::new(64);
        let sty_depth = stygian::DepthImage::new(vec2(64, 64));

        let p1 = sty_model_matrix.transform_point(Point3::new(
            sty_terrain.size().x as f32 * 0.5,
            sty_terrain.size().y as f32 * 0.5,
            sty_terrain.size().z as f32,
        ));
        let p2 = sty_model_matrix.transform_point(Point3::new(
            sty_terrain.size().x as f32 * 0.6,
            sty_terrain.size().y as f32 * 0.6,
            sty_terrain.size().z as f32 * 5.0,
        ));
        let birds_eye_view_matrix = Matrix4::look_at(p2, p1, vec3(0.0, 0.0, 1.0));

        Self {
            imgui_renderer,

            scene_renderer,
            scene_instance,

            sty_terrain,
            sty_rast,
            sty_depth,
            sty_model_matrix,

            linedraw: lib::linedraw::LineDraw::new(facade),

            depthvis: lib::depthvis::DepthVis::new(facade),

            birds_eye_view_matrix,
        }
    }

    fn render(
        &mut self,
        facade: &impl Facade,
        state: &State,
        ui: imgui::Ui,
        metrics: &mut Metrics,
        target: &mut impl Surface,
    ) {
        let params = state.render_params(4.0 / 3.0);

        // Set up the Stygian internal state tracing for visualization
        use std::cell::RefCell;
        struct Log {
            samples: Vec<([Vector3<f32>; 4], f32)>,
        }
        let mut log = RefCell::new(Log {
            samples: Vec::new(),
        });

        #[derive(Clone)]
        struct Tracer<'a>(&'a RefCell<Log>);
        impl stygian::Trace for Tracer<'_> {
            fn wants_opticast_sample(&mut self) -> bool {
                true
            }
            fn opticast_sample(&mut self, vertices: &[Vector3<f32>; 4], depth: f32) {
                self.0.borrow_mut().samples.push((*vertices, depth));
            }
        }

        // Update Stygian
        let sty_depth_build_start = Instant::now();
        {
            self.sty_rast.set_camera_matrix_trace(
                params.camera_matrix * self.sty_model_matrix,
                Tracer(&log),
            );

            if state.show_opticast_samples {
                self.sty_rast
                    .update_with_trace(&self.sty_terrain, Tracer(&log));
            } else {
                self.sty_rast.update_with(&self.sty_terrain);
            }

            self.sty_rast.rasterize_to(&mut self.sty_depth);
        }
        metrics.log_depth_build_time(sty_depth_build_start.elapsed().as_float_secs() as f32);

        // Render a scene
        target.clear_color_and_depth((0.3, 0.3, 0.5, 1.0), 0.0);

        let vp_matrix = Matrix4::from_nonuniform_scale(0.9, 0.9, 1.0);

        let camera_matrix = vp_matrix * params.camera_matrix;
        let query = stygian::QueryContext::new(&self.sty_depth);
        let mut num_objects = 0;
        let mut num_rendered_objects = 0;

        let draw_params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::DepthTest::IfMore,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };

        self.scene_renderer.draw_scene(
            &self.scene_instance,
            target,
            &draw_params,
            camera_matrix,
            |transform, ms_aabb| {
                let visible =
                    query.query_cs_aabb(transform_aabb(ms_aabb, params.camera_matrix * transform));

                num_objects += 1;
                num_rendered_objects += visible as usize;

                visible
            },
        );

        metrics.log_num_objects(num_rendered_objects, num_objects);

        if state.show_birds_eye_view {
            use cgmath::{PerspectiveFov, Rad};

            let proj: Matrix4<f32> = PerspectiveFov {
                fovy: Rad(1.0),
                aspect: 4.0 / 3.0,
                near: 0.5,
                far: 2000.0,
            }
            .into();

            let camera_matrix = Matrix4::from_translation(vec3(0.0, 0.0, 0.5))
                * Matrix4::from_nonuniform_scale(1.0, 1.0, -0.5)
                * proj
                * self.birds_eye_view_matrix;

            let (w, h) = target.get_dimensions();
            let rect = glium::Rect {
                left: w * 5 / 100,
                bottom: h * 5 / 100,
                width: w * 30 / 100,
                height: h * 30 / 100,
            };

            target.clear(
                Some(&rect),
                Some((0.5, 0.3, 0.3, 1.0)),
                true,
                Some(0.0),
                None,
            );

            let draw_params = glium::DrawParameters {
                depth: glium::Depth {
                    test: glium::DepthTest::IfMore,
                    write: true,
                    ..Default::default()
                },
                viewport: Some(rect),
                ..Default::default()
            };

            // Visible
            self.scene_renderer.draw_scene(
                &self.scene_instance,
                target,
                &draw_params,
                camera_matrix,
                |transform, ms_aabb| {
                    query.query_cs_aabb(transform_aabb(ms_aabb, params.camera_matrix * transform))
                },
            );

            let draw_params = glium::DrawParameters {
                polygon_mode: glium::PolygonMode::Line,
                ..draw_params
            };

            // Occluded but not frustum culled
            let sty_empty_depth = stygian::DepthImage::new(vec2(1, 1));
            let empty_query = stygian::QueryContext::new(&sty_empty_depth);
            self.scene_renderer.draw_scene(
                &self.scene_instance,
                target,
                &draw_params,
                camera_matrix,
                |transform, ms_aabb| {
                    let cs_aabb = transform_aabb(ms_aabb, params.camera_matrix * transform);
                    empty_query.query_cs_aabb(cs_aabb) && !query.query_cs_aabb(cs_aabb)
                },
            );
        }

        // Draw a HUD
        self.linedraw.push(
            [0, 0, 0, 255],
            [
                [-1.0, -1.0],
                [1.0, -1.0],
                [1.0, 1.0],
                [-1.0, 1.0],
                [-1.0, -1.0],
            ]
            .iter()
            .map(|x| trans_point2(vp_matrix, *x)),
        );

        if state.show_opticast_samples {
            let m = camera_matrix * self.sty_model_matrix;
            for (verts, depth) in log.get_mut().samples.iter() {
                use array::Array4;

                let verts = verts.map(|v| {
                    let p = m * v.extend(0.0);
                    Point2::new(p.x / p.w, p.y / p.w)
                });

                // Make polygons slightly smaller
                let verts = verts.map(|v| v + (verts[0] - v) * 0.1);

                // Color by depth
                let color = scalar_to_color(1.0 - 0.1 / (*depth + 0.1));

                self.linedraw.push(
                    color,
                    [verts[0], verts[1], verts[2], verts[3], verts[0]]
                        .iter()
                        .cloned(),
                );
            }
        }

        self.linedraw.flush(target);

        if state.show_depth {
            self.depthvis.draw(facade, target, &self.sty_depth);
        }

        self.imgui_renderer.render(target, ui).unwrap();

        metrics.fps_counter.log(1.0);
    }
}

fn trans_point2(m: Matrix4<f32>, p: impl Into<Point2<f32>>) -> Point2<f32> {
    let p = p.into();
    let p = m.transform_point(Point3::new(p.x, p.y, 0.0));
    Point2::new(p.x, p.y)
}

fn scalar_to_color(x: f32) -> [u8; 4] {
    //   0 1 2 3 4 5 6 7
    // R   1 1       1 1
    // G     1 1 1     1
    // B         1 1 1 1
    let r_map = [0, 1, 1, 0, 0, 0, 1, 1, 1];
    let g_map = [0, 0, 1, 1, 1, 0, 0, 1, 1];
    let b_map = [0, 0, 0, 0, 1, 1, 1, 1, 1];

    let x = x.max(0.0).min(1.0) * 7.0;
    let i = x as usize;
    let f = x - i as f32;

    let r = r_map[i] as f32 * (1.0 - f) + r_map[i + 1] as f32 * f;
    let g = g_map[i] as f32 * (1.0 - f) + g_map[i + 1] as f32 * f;
    let b = b_map[i] as f32 * (1.0 - f) + b_map[i + 1] as f32 * f;

    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255]
}

fn transform_aabb(aabb: [Vector4<f32>; 2], m: Matrix4<f32>) -> [Vector4<f32>; 2] {
    debug_assert_eq!(aabb[0].w, aabb[1].w);

    let p = [
        m * vec4(aabb[0].x, aabb[0].y, aabb[0].z, aabb[0].w),
        m * vec4(aabb[0].x, aabb[0].y, aabb[1].z, aabb[0].w),
        m * vec4(aabb[0].x, aabb[1].y, aabb[0].z, aabb[0].w),
        m * vec4(aabb[0].x, aabb[1].y, aabb[1].z, aabb[0].w),
        m * vec4(aabb[1].x, aabb[0].y, aabb[0].z, aabb[0].w),
        m * vec4(aabb[1].x, aabb[0].y, aabb[1].z, aabb[0].w),
        m * vec4(aabb[1].x, aabb[1].y, aabb[0].z, aabb[0].w),
        m * vec4(aabb[1].x, aabb[1].y, aabb[1].z, aabb[0].w),
    ];

    [
        vec4(
            [
                p[0].x, p[1].x, p[2].x, p[3].x, p[4].x, p[5].x, p[6].x, p[7].x,
            ]
            .fmin(),
            [
                p[0].y, p[1].y, p[2].y, p[3].y, p[4].y, p[5].y, p[6].y, p[7].y,
            ]
            .fmin(),
            [
                p[0].z, p[1].z, p[2].z, p[3].z, p[4].z, p[5].z, p[6].z, p[7].z,
            ]
            .fmin(),
            [
                p[0].w, p[1].w, p[2].w, p[3].w, p[4].w, p[5].w, p[6].w, p[7].w,
            ]
            .fmin(),
        ),
        vec4(
            [
                p[0].x, p[1].x, p[2].x, p[3].x, p[4].x, p[5].x, p[6].x, p[7].x,
            ]
            .fmax(),
            [
                p[0].y, p[1].y, p[2].y, p[3].y, p[4].y, p[5].y, p[6].y, p[7].y,
            ]
            .fmax(),
            [
                p[0].z, p[1].z, p[2].z, p[3].z, p[4].z, p[5].z, p[6].z, p[7].z,
            ]
            .fmax(),
            [
                p[0].w, p[1].w, p[2].w, p[3].w, p[4].w, p[5].w, p[6].w, p[7].w,
            ]
            .fmax(),
        ),
    ]
}
