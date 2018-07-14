//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate cgmath;
extern crate ngspf;
extern crate ttf_noto_sans;
#[macro_use]
extern crate attrtext;
extern crate lipsum;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use cgmath::prelude::*;
use cgmath::{vec3, Matrix4, Point2, Vector2};

use ngspf::{
    canvas::{painter::new_painter_for_image_data, text, ImageData, ImageFormat, ImageRef},
    cggeom::{prelude::*, Box2},
    core::GroupRef,
    prelude::*,
    viewport::rgb::RGBA,
    viewport::{
        ImageWrapMode, LayerBuilder, LayerContents, LayerRef, RootRef, VirtualKeyCode,
        WindowBuilder, WindowEvent, WindowFlagsBit, WindowRef, WorkspaceBuilder,
    },
};

fn render_second_image(font_config: &text::FontConfig, extents: [usize; 2]) -> ImageData {
    let mut image_data =
        ImageData::new(Vector2::new(extents[0], extents[1]), ImageFormat::SrgbRgba8);
    {
        let mut painter = new_painter_for_image_data(&mut image_data);

        let mut para_style = text::ParagraphStyle::new();
        para_style.text_align = text::TextAlign::Justify;

        let body = text::CharStyle::default();
        let emph = text::CharStyle {
            color: Some(RGBA::new(1.0, 1.0, 0.0, 1.0)),
            ..Default::default()
        };

        let lipsum = lipsum::lipsum(100);
        let text = text! {{ body; {emph; ("Example text\n")} (lipsum.as_str()) }};

        let boundary = text::BoxBoundary::new(0.0..extents[0] as f64);

        let layout = font_config.layout_area_text(&text, &para_style, &boundary);

        painter.set_fill_color(RGBA::new(1.0, 1.0, 1.0, 1.0));

        painter.save();
        painter.translate(Vector2::new(0.0, 0.0));
        painter.fill_text_layout(&layout, &font_config);
        painter.restore();
    }

    image_data
}

fn main() {
    let mut ws = WorkspaceBuilder::new()
        .application_name("NgsPF Example: text")
        .application_version(1, 0, 0)
        .build()
        .expect("failed to create a workspace");
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
        image_data = ImageData::new(Vector2::new(480, 480), ImageFormat::SrgbRgba8Premul);
        let mut painter = new_painter_for_image_data(&mut image_data);

        let para_style = text::ParagraphStyle::new();

        let body = text::CharStyle::default();
        let emph = text::CharStyle {
            color: Some(RGBA::new(1.0, 1.0, 0.0, 1.0)),
            ..Default::default()
        };

        let text = text! {{ body; {emph; ("Hello")} (", world! مرحبا ") }};

        let layout = font_config.layout_point_text(&text, &para_style);

        painter.set_fill_color(RGBA::new(1.0, 1.0, 1.0, 1.0));

        painter.save();
        painter.translate(Vector2::new(40.0, 60.0));
        painter.scale(1.0);
        painter.fill_text_layout(&layout, &font_config);
        painter.restore();

        painter.save();
        painter.translate(Vector2::new(40.0, 150.0));
        painter.scale(4.0);
        painter.fill_text_layout(&layout, &font_config);
        painter.restore();

        painter.save();
        painter.translate(Vector2::new(40.0, 400.0));
        painter.scale(16.0);
        painter.fill_text_layout(&layout, &font_config);
        painter.restore();
    }

    // Produce the first frame
    let root = RootRef::clone(ws.root());
    let window: WindowRef;
    let dyn_layer: LayerRef;
    {
        let image_ref = ImageRef::new_immutable(image_data);

        let image = LayerBuilder::new()
            .contents(LayerContents::Image {
                image: image_ref,
                source: Box2::new(Point2::origin(), Point2::new(480.0, 480.0)),
                wrap_mode: ImageWrapMode::Repeat,
            })
            .bounds(Box2::new(Point2::origin(), Point2::new(480.0, 480.0)))
            .transform(Matrix4::from_translation(vec3(0.0, 0.0, 0.0)))
            .build(&context);

        let image_ref = ImageRef::new_immutable(render_second_image(&font_config, [160, 480]));

        dyn_layer = LayerBuilder::new()
            .contents(LayerContents::Image {
                image: image_ref,
                source: Box2::new(Point2::origin(), Point2::new(160.0, 480.0)),
                wrap_mode: ImageWrapMode::Repeat,
            })
            .bounds(Box2::new(Point2::origin(), Point2::new(160.0, 480.0)))
            .transform(Matrix4::from_translation(vec3(480.0, 0.0, 0.0)))
            .build(&context);

        let group = GroupRef::new(
            [&image, &dyn_layer]
                .iter()
                .cloned()
                .cloned()
                .map(LayerRef::into_node_ref),
        );

        window = WindowBuilder::new()
            .flags(WindowFlagsBit::Resizable)
            .child(Some(group.into_node_ref()))
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
                let mut new_size = None;

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
                        WindowEvent::Resized(size) => {
                            new_size = Some([
                                (size.x.max(481.0) - 480.0) as usize,
                                size.y.max(1.0) as usize,
                            ]);
                        }
                        _ => {}
                    }
                }

                {
                    let mut frame = context
                        .lock_producer_frame()
                        .expect("failed to acquire a producer frame");

                    if let Some(new_size) = new_size {
                        // Re-render the contents of `dyn_layer`
                        let new_image = render_second_image(&font_config, new_size);

                        // Set the new image
                        let size = new_image.size().cast::<f32>().unwrap();
                        let image_ref = ImageRef::new_immutable(new_image);
                        dyn_layer
                            .contents()
                            .set(
                                &mut frame,
                                LayerContents::Image {
                                    image: image_ref,
                                    source: Box2::new(
                                        Point2::origin(),
                                        Point2::new(size.x, size.y),
                                    ),
                                    wrap_mode: ImageWrapMode::Repeat,
                                },
                            )
                            .unwrap();
                        dyn_layer
                            .bounds()
                            .set(
                                &mut frame,
                                Box2::new(Point2::origin(), Point2::new(size.x, size.y)),
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
