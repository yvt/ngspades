//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate cgmath;
extern crate ngspf;

use std::sync::Arc;

use cgmath::Vector2;

use ngspf::viewport::{Workspace, WindowBuilder, LayerBuilder, ImageRef, ImageData, ImageFormat,
                      LayerContents, WindowFlagsBit};
use ngspf::prelude::*;

static IMAGE: &[u8] = include_bytes!("../../ngsgfx/examples/nyancat.raw");

fn main() {
    let mut ws = Workspace::new().expect("failed to create a workspace");
    let context = Arc::clone(ws.context());

    // Produce the first frame
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
            .contents(LayerContents::Image(image_ref))
            .build(&context);

        let window = WindowBuilder::new()
            .flags(WindowFlagsBit::Resizable.into())
            .child(Some(layer.into_node_ref()))
            .build(&context);

        let mut frame = context.lock_producer_frame().expect(
            "failed to acquire a producer frame",
        );
        ws.root()
            .windows()
            .set(&mut frame, Some(window.into_node_ref()))
            .expect("failed to set the value of proeprty 'windows'");
    }
    context.commit().expect("failed to commit a frame");

    // Start the main loop
    ws.enter_main_loop().expect(
        "error occured while running the main loop",
    );
}
