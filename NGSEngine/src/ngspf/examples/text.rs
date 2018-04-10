//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate cgmath;
extern crate ngspf;
extern crate ttf_noto_sans;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use cgmath::prelude::*;
use cgmath::{Matrix4, Point2, Vector2, vec3};

use ngspf::{canvas::{ImageData, ImageFormat, ImageRef, painter::new_painter_for_image_data,
                     text},
            ngsbase::{Box2, prelude::*}, prelude::*, viewport::rgb::RGBA,
            viewport::{ImageWrapMode, LayerBuilder, LayerContents, RootRef,
                       VirtualKeyCode, WindowBuilder, WindowEvent, WindowFlagsBit, WindowRef,
                       Workspace}};

fn main() {
    let mut ws = Workspace::new().expect("failed to create a workspace");
    let context = Arc::clone(ws.context());
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);

    // Create font config
    let mut font_config;
    {
        const BEHDAD_REGULAR: &[u8] =
            include_bytes!("../src/canvas/tests/fonts/Behdad-Regular.otf");

        font_config = text::FontConfig::new();
        font_config.insert(
            &text::Font::new(BEHDAD_REGULAR).unwrap(),
            0,
            "Behdad",
            text::FontStyle::Normal,
            400,
        );
        font_config.insert(
            &text::Font::new(ttf_noto_sans::REGULAR).unwrap(),
            0,
            "Noto Sans",
            text::FontStyle::Normal,
            400,
        );
    }

    // Create image
    let mut image_data;
    {
        image_data = ImageData::new(Vector2::new(640, 480), ImageFormat::SrgbRgba8);
        let mut painter = new_painter_for_image_data(&mut image_data);

        let para_style = text::ParagraphStyle::new();

        let layout = font_config
            .layout_point_text([("Hello, world! مرحبا ", ())][..].into(), &para_style);

        painter.translate(Vector2::new(160.0, 240.0));
        painter.set_fill_color(RGBA::new(1.0, 1.0, 1.0, 1.0));
        painter.fill_text_layout(&layout, &font_config, false);
    }

    // Produce the first frame
    let root = RootRef::clone(ws.root());
    let window: WindowRef;
    {
        let image_ref = ImageRef::new_immutable(image_data);

        let image = LayerBuilder::new()
            .contents(LayerContents::Image {
                image: image_ref,
                source: Box2::new(Point2::origin(), Point2::new(640.0, 480.0)),
                wrap_mode: ImageWrapMode::Repeat,
            })
            .bounds(Box2::new(Point2::origin(), Point2::new(640.0, 480.0)))
            .transform(Matrix4::from_translation(vec3(0.0, 0.0, 0.0)))
            .build(&context);

        window = WindowBuilder::new()
            .flags(WindowFlagsBit::Resizable)
            .child(Some(image.into_node_ref()))
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
                        _ => {}
                    }
                }

                {
                    let mut frame = context
                        .lock_producer_frame()
                        .expect("failed to acquire a producer frame");

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
