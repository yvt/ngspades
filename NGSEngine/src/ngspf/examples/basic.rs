//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate cgmath;
extern crate ngspf;

use std::thread;
use std::sync::{Arc, mpsc, Mutex};

use cgmath::{Vector2, Point2};
use cgmath::prelude::*;

use ngspf::viewport::{Workspace, WindowBuilder, LayerBuilder, ImageRef, ImageData, ImageFormat,
                      LayerContents, WindowFlagsBit, WindowRef, WindowEvent, RootRef,
                      ImageWrapMode};
use ngspf::prelude::*;
use ngspf::ngsbase::Box2;
use ngspf::ngsbase::prelude::*;

static IMAGE: &[u8] = include_bytes!("../../ngsgfx/examples/nyancat.raw");

fn main() {
    let mut ws = Workspace::new().expect("failed to create a workspace");
    let context = Arc::clone(ws.context());
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);

    // Produce the first frame
    let root = RootRef::clone(ws.root());
    let window: WindowRef;
    {
        let mut image_data = ImageData::new(Vector2::new(128, 128), ImageFormat::SrgbRgba8);
        for i in 0..128 * 128 {
            let rgba = &IMAGE[i * 4..];
            image_data.pixels_u32_mut()[i] = rgba[0] as u32 | ((rgba[1] as u32) << 8) |
                ((rgba[2] as u32) << 16) |
                ((rgba[3] as u32) << 24);
        }
        let image_ref = ImageRef::new_immutable(image_data);

        let layer = LayerBuilder::new()
            .contents(LayerContents::Image {
                image: image_ref,
                source: Box2::new(Point2::origin(), Point2::new(128.0, 128.0)),
                wrap_mode: ImageWrapMode::Repeat,
            })
            .bounds(Box2::new(Point2::origin(), Point2::new(128.0, 128.0)))
            .build(&context);

        window = WindowBuilder::new()
            .flags(WindowFlagsBit::Resizable)
            .child(Some(layer.clone().into_node_ref()))
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
            let mut i = 0;
            let mut exit = false;
            while !exit {
                i += 1;

                // Process window events
                for event in rx.try_iter() {
                    match event {
                        WindowEvent::Close => {
                            exit = true;
                        }
                        _ => {}
                    }
                }

                {
                    let mut frame = context.lock_producer_frame().expect(
                        "failed to acquire a producer frame",
                    );

                    window
                        .title()
                        .set(&mut frame, format!("frame = {}", i))
                        .unwrap();

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
    ws.enter_main_loop().expect(
        "error occured while running the main loop",
    );
}
