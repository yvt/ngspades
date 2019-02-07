//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{conv::array4x4, prelude::*, vec2, vec3, Matrix3, Matrix4, Point2, Point3, Vector3};
use glium::{
    backend::Facade, glutin, program, uniform, IndexBuffer, Program, Surface, VertexBuffer,
};
use std::time::Instant;

use stygian;

#[path = "../common/terrainload.rs"]
mod terrainload;

mod lib {
    pub mod cube;
    pub mod linedraw;
    pub mod vxl2mesh;
}

fn main() {
    use clap::{App, Arg};
    // Use `clap` to parse command-line arguments
    let matches = App::new("stygian_demo1")
        .about("Stygian demo app 1")
        .arg(
            Arg::with_name("INPUT")
                .help("file to display; .vxl and .vox formats are supported")
                .index(1),
        )
        .get_matches();

    // Load the input vox file
    println!("Loading the input file");
    let terrain = if let Some(input_path) = matches.value_of_os("INPUT") {
        terrainload::load_terrain(input_path)
    } else {
        terrainload::DERBY_RACERS.clone()
    };

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new();
    let context = glutin::ContextBuilder::new()
        .with_depth_buffer(24)
        .with_vsync(true);
    let display = glium::Display::new(window, context, &events_loop).unwrap();
    let mut renderer = Renderer::new(&display, terrain);

    let mut state = State::new(&renderer.terrain);

    use glutin::{ElementState, Event, VirtualKeyCode, WindowEvent};

    let mut last_time = Instant::now();

    let mut keep_running = true;

    while keep_running {
        events_loop.poll_events(|event| match event {
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
        });

        let delta_time = last_time.elapsed();
        last_time = Instant::now();

        let delta_time = delta_time.subsec_nanos() as f32 * 1.0e-9;

        state.update(delta_time);

        // drawing a frame
        let mut target = display.draw();
        renderer.render(&state.render_params(4.0 / 3.0), &mut target);

        target.finish().unwrap();

        let title = format!(
            "Stygian demo app 1 [{:.2} fps]",
            renderer.fps_counter.rate(),
        );
        display.gl_window().set_title(&title);
    }
}

#[derive(Debug)]
struct State {
    eye: Vector3<f32>,
    velocity: Vector3<f32>,
    angle: Vector3<f32>,
    angular_velocity: Vector3<f32>,
    keys: [bool; 16],
}

impl State {
    fn new(terrain: &ngsterrain::Terrain) -> State {
        let size = terrain.size();

        let eye_xy = vec2(size.x / 2, size.y / 2);
        let floor = (terrain.get_row(eye_xy).unwrap())
            .chunk_z_ranges()
            .last()
            .unwrap()
            .end;
        let eye_z = floor + size.z / 10 + 1;

        State {
            eye: vec3(eye_xy.x, eye_xy.y, eye_z).cast::<f32>().unwrap(),
            velocity: Vector3::zero(),
            angle: vec3(-0.4, 0.0, 0.0),
            angular_velocity: Vector3::zero(),
            keys: [false; 16],
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

    fn view_matrix(&self) -> Matrix4<f32> {
        use cgmath::{vec4, Basis3, Rad};
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
            camera_matrix: Matrix4::from_translation(vec3(0.0, 0.0, 1.0))
                * Matrix4::from_nonuniform_scale(1.0, 1.0, -1.0)
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
struct RenderParams {
    camera_matrix: Matrix4<f32>,
}

struct Renderer {
    terrain: ngsterrain::Terrain,
    terrain_vb: VertexBuffer<lib::vxl2mesh::TerrainVertex>,
    terrain_ib: IndexBuffer<u32>,
    terrain_program: Program,

    sty_terrain: stygian::Terrain,
    sty_rast: stygian::TerrainRast,
    sty_depth: stygian::DepthImage,

    linedraw: lib::linedraw::LineDraw,

    fps_counter: PerfCounter,
}

impl Renderer {
    fn new(facade: &impl Facade, terrain: ngsterrain::Terrain) -> Self {
        // Convert the terrain to a mesh
        println!("Converting the terrain into a mesh");
        let terrain_vb;
        let terrain_ib;
        let terrain_program;
        {
            use self::lib::vxl2mesh;
            let (verts, indices) = vxl2mesh::terrain_to_mesh(&terrain);
            terrain_vb = VertexBuffer::new(facade, &verts).unwrap();
            terrain_ib =
                IndexBuffer::new(facade, glium::index::PrimitiveType::TrianglesList, &indices)
                    .unwrap();
            terrain_program = program!(facade,
            100 => {
                vertex: r"
                    #version 100

                    uniform highp mat4 u_matrix;
                    attribute highp vec3 pos;
                    attribute highp vec3 norm;
                    attribute highp vec4 color;
                    varying lowp vec4 v_color;

                    void main() {
                        v_color = color / 255.0;
                        v_color *= sqrt(dot(norm, normalize(vec3(0.3, 0.7, 0.8))) * 0.5 + 0.5);
                        gl_Position = u_matrix * vec4(pos, 1.0);
                    }
                ",
                fragment: r"
                    #version 100

                    varying lowp vec4 v_color;

                    void main() {
                        gl_FragColor = v_color;
                    }
                ",
            })
            .unwrap();
        }

        println!("Initializing Stygian");
        let sty_terrain = stygian::Terrain::from_ngsterrain(&terrain).unwrap();
        let sty_rast = stygian::TerrainRast::new(64);
        let sty_depth = stygian::DepthImage::new(vec2(64, 64));

        Self {
            terrain,
            terrain_vb,
            terrain_ib,
            terrain_program,

            sty_terrain,
            sty_rast,
            sty_depth,

            linedraw: lib::linedraw::LineDraw::new(facade),

            fps_counter: PerfCounter::new(),
        }
    }

    fn render(&mut self, params: &RenderParams, target: &mut impl Surface) {
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
        self.sty_rast
            .set_camera_matrix_trace(params.camera_matrix, Tracer(&log));

        self.sty_rast
            .rasterize_trace(&self.sty_terrain, &mut self.sty_depth, Tracer(&log));

        // Render a scene
        target.clear_color_and_depth((0.5, 0.5, 0.5, 1.0), 0.0);

        let vp_matrix = Matrix4::from_nonuniform_scale(0.9, 0.9, 1.0);

        let camera_matrix = vp_matrix * params.camera_matrix;

        let uniforms = uniform! {
            u_matrix: array4x4(camera_matrix),
        };

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::DepthTest::IfMore,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };

        target
            .draw(
                &self.terrain_vb,
                &self.terrain_ib,
                &self.terrain_program,
                &uniforms,
                &params,
            )
            .unwrap();

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

        for (verts, depth) in log.get_mut().samples.iter() {
            use array::Array4;

            let verts = verts.map(|v| {
                let p = camera_matrix * v.extend(0.0);
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

        self.linedraw.flush(target);

        self.fps_counter.log(1.0);
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
