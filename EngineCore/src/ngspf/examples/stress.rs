//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Displays and animates a crazy number of layers.
//! Use arrow keys to adjust the layer count.
extern crate cgmath;
extern crate ngspf;
extern crate rand;

use std::thread;
use std::sync::{Arc, mpsc, Mutex};

use cgmath::{Point2, Vector3, Matrix4, vec3};

use ngspf::core::{Context, GroupRef, ProducerFrame};
use ngspf::viewport::{WorkspaceBuilder, WindowBuilder, LayerBuilder, LayerContents, WindowFlags,
                      WindowRef, WindowEvent, RootRef, LayerRef, VirtualKeyCode};
use ngspf::prelude::*;
use ngspf::cggeom::Box2;
use ngspf::cggeom::prelude::*;
use ngspf::viewport::rgb::RGBA;

struct LorenzSystem {
    rho: f32,
    sigma: f32,
    beta: f32,
}

impl LorenzSystem {
    fn dxdt(&self, p: Vector3<f32>) -> Vector3<f32> {
        vec3(
            self.sigma * (p.y - p.x),
            p.x * (self.rho - p.z) - p.y,
            p.x * p.y - self.beta * p.z,
        )
    }
}

struct State {
    context: Arc<Context>,
    points: Vec<Vector3<f32>>,
    layers: Vec<Vec<LayerRef>>,
    system: LorenzSystem,
    root: LayerRef,
    rng: rand::XorShiftRng,
    trail_i: usize,
}

impl State {
    fn new(context: Arc<Context>) -> Self {
        Self {
            points: Vec::new(),
            layers: Vec::new(),
            root: LayerBuilder::new()
                .transform(Matrix4::from_translation(vec3(320.0, 240.0, 0.0)))
                .build(&context),
            system: LorenzSystem {
                rho: 28.0,
                sigma: 10.0,
                beta: 8.0 / 3.0,
            },
            rng: rand::weak_rng(),
            trail_i: 0,
            context,
        }
    }

    fn resize(&mut self, frame: &mut ProducerFrame, num_points: usize, trail_len: usize) {
        // Resize `self.points`
        use rand::distributions::IndependentSample;
        let dist = rand::distributions::Normal::new(0.0, 10.0);
        self.points.truncate(num_points);
        for _ in self.points.len()..num_points {
            self.points.push(
                vec3(
                    dist.ind_sample(&mut self.rng),
                    dist.ind_sample(&mut self.rng),
                    dist.ind_sample(&mut self.rng),
                ).cast(),
            );
        }

        // Resize `self.layers`
        let c = RGBA::new(1.0, 1.0, 1.0, 0.05);
        self.layers.resize(num_points, Vec::new());
        for ls in self.layers.iter_mut() {
            ls.truncate(trail_len);
            for _ in ls.len()..trail_len {
                ls.push(
                    LayerBuilder::new()
                        .contents(LayerContents::Solid(c))
                        .bounds(Box2::new(Point2::new(-2.0, -2.0), Point2::new(2.0, 2.0)))
                        .transform(Matrix4::from_translation(vec3(-100.0, -100.0, 0.0)))
                        .build(&self.context),
                )
            }
        }

        // Update the contents of `self.root`
        let layers = self.layers.concat();
        self.root
            .child()
            .set(
                frame,
                Some(
                    GroupRef::new(layers.into_iter().map(LayerRef::into_node_ref)).into_node_ref(),
                ),
            )
            .unwrap();

        if self.trail_i >= trail_len {
            self.trail_i = 0;
        }
    }

    fn update_points(&mut self, frame: &mut ProducerFrame) {
        for p in self.points.iter_mut() {
            let dtdx = self.system.dxdt(*p);
            *p += dtdx * 0.001;
        }
        for (p, ls) in self.points.iter().zip(self.layers.iter()) {
            let mut p = p * 10.0;
            p.z = 0.0;
            let m = Matrix4::from_translation(p);
            ls[self.trail_i].transform().set(frame, m).unwrap();
        }
        self.trail_i = (self.trail_i + 1) % self.layers[0].len();
    }
}

fn main() {
    let mut ws = WorkspaceBuilder::new()
        .application_name("NgsPF Example: stress")
        .application_version(1, 0, 0)
        .build()
        .expect("failed to create a workspace");
    let context = Arc::clone(ws.context());
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);

    // Setup the animation controller
    let mut num_points = 64;
    let mut trail_len = 10;
    let mut state = State::new(Arc::clone(ws.context()));
    {
        let mut frame = context.lock_producer_frame().expect(
            "failed to acquire a producer frame",
        );
        state.resize(&mut frame, num_points, trail_len);
        state.update_points(&mut frame);
    }

    // Produce the first frame
    let root = RootRef::clone(ws.root());
    let window: WindowRef;
    {
        window = WindowBuilder::new()
            .flags(WindowFlags::empty())
            .child(Some(state.root.clone().into_node_ref()))
            .listener(Some(Box::new(move |event| {
                // Send the event to the producer loop
                let _ = tx.lock().unwrap().send(event.clone());
            })))
            .build(&context);

        let mut frame = context.lock_producer_frame().expect(
            "failed to acquire a producer frame",
        );
        ws.root()
            .windows()
            .set(&mut frame, Some(window.clone().into_node_ref()))
            .expect("failed to set the value of proeprty 'windows'");
    }
    context.commit().expect("failed to commit a frame");

    // Start the producer loop
    thread::Builder::new()
        .spawn(move || {
            use std::time::Duration;
            let mut exit = false;
            let mut update = true;
            while !exit {
                // Process window events
                for event in rx.try_iter() {
                    match event {
                        WindowEvent::Close => {
                            exit = true;
                        }
                        WindowEvent::KeyboardInput(vk, pressed, _) => {
                            if pressed {
                                match vk {
                                    VirtualKeyCode::Escape => {
                                        exit = true;
                                    }
                                    VirtualKeyCode::Down => {
                                        if num_points > 1 {
                                            num_points /= 2;
                                        }
                                        update = true;
                                    }
                                    VirtualKeyCode::Up => {
                                        num_points *= 2;
                                        update = true;
                                    }
                                    VirtualKeyCode::Left => {
                                        if trail_len > 1 {
                                            trail_len -= 1;
                                        }
                                        update = true;
                                    }
                                    VirtualKeyCode::Right => {
                                        trail_len += 1;
                                        update = true;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Limit the producer frame rate (currently there is no proper
                // mechanism to do this)
                if context.num_pending_frames() > 2 && !exit {
                    thread::sleep(Duration::from_millis(15));
                    continue;
                }

                {
                    let mut frame = context.lock_producer_frame().expect(
                        "failed to acquire a producer frame",
                    );

                    if update {
                        state.resize(&mut frame, num_points, trail_len);
                        window
                            .title()
                            .set(
                                &mut frame,
                                format!("points = {}, trail len = {}", num_points, trail_len),
                            )
                            .unwrap();
                        update = false;
                    }
                    state.update_points(&mut frame);

                    if exit {
                        root.exit_loop(&mut frame).unwrap();
                    }
                }
                context.commit().expect("failed to commit a frame");
                thread::sleep(Duration::from_millis(10));
            }
        })
        .unwrap();

    // Start the main loop
    ws.enter_main_loop().expect(
        "error occured while running the main loop",
    );
}
