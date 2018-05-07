//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate cgmath;
extern crate ngsgamegfx;
extern crate ngspf;
extern crate refeq;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use cgmath::prelude::*;
use cgmath::{Matrix4, Point2};

use refeq::RefEqArc;

use ngspf::cggeom::prelude::*;
use ngspf::cggeom::Box2;
use ngspf::prelude::*;
use ngspf::viewport::{
    LayerBuilder, LayerContents, LayerRef, RootRef, VirtualKeyCode, WindowBuilder, WindowEvent,
    WindowFlagsBit, WindowRef, WorkspaceBuilder,
};

fn main() {
    let mut ws = WorkspaceBuilder::new()
        .application_name("NgsGameGFX example: basic")
        .build()
        .expect("failed to create a workspace");
    let context = Arc::clone(ws.context());
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);

    // Produce the first frame
    let root = RootRef::clone(ws.root());
    let window: WindowRef;
    let image: LayerRef;
    let port = ngsgamegfx::port::PortRef::new(&context);
    {
        image = LayerBuilder::new()
            .contents(LayerContents::Port(RefEqArc::new(port.clone())))
            .bounds(Box2::new(Point2::origin(), Point2::new(640.0, 480.0)))
            .transform(Matrix4::identity())
            .build(&context);

        window = WindowBuilder::new()
            .flags(WindowFlagsBit::Resizable)
            .child(Some(image.clone().into_node_ref()))
            .listener(Some(Box::new(move |event| {
                // Send the event to the producer loop
                let _ = tx.lock().unwrap().send(event.clone());
            })))
            .build(&context);

        let mut frame = context
            .lock_producer_frame()
            .expect("failed to acquire a producer frame");
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
            while !exit {
                // Process window events
                let mut new_size = None;
                for event in rx.try_iter() {
                    match event {
                        WindowEvent::Close => {
                            exit = true;
                        }
                        WindowEvent::KeyboardInput(vk, pressed, _) => {
                            if pressed && vk == VirtualKeyCode::Escape {
                                exit = true;
                            }
                        }
                        WindowEvent::Resized(size) => {
                            new_size = Some(size);
                        }
                        _ => {}
                    }
                }

                {
                    let mut frame = context
                        .lock_producer_frame()
                        .expect("failed to acquire a producer frame");

                    if let Some(x) = new_size {
                        image
                            .bounds()
                            .set(
                                &mut frame,
                                Box2::new(Point2::origin(), Point2::new(x.x, x.y)),
                            )
                            .unwrap();
                    }

                    if exit {
                        root.exit_loop(&mut frame).unwrap();
                    }
                }
                context.commit().expect("failed to commit a frame");
                thread::sleep(Duration::from_millis(15));
            }
        })
        .unwrap();

    // Start the main loop
    ws.enter_main_loop()
        .expect("error occured while running the main loop");
}
