//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{utils, TestDriver};
use gfx;
use gfx::prelude::*;

static SPIRV_VERT: ::include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/render_null.vert.spv"));

static SPIRV_FRAG: ::include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/render_null.frag.spv"));

// Execute an emoty rendering pipeline.
pub fn render_null<T: TestDriver>(driver: T) {
    driver.for_each_render_queue(&mut |device, qf| {
        println!("- Creating a command queue");
        let queue = device.build_cmd_queue().queue_family(qf).build().unwrap();

        println!("- Creating libraries");
        let library_frag = device.new_library(SPIRV_FRAG.as_u32_slice()).unwrap();
        let library_vert = device.new_library(SPIRV_VERT.as_u32_slice()).unwrap();

        println!("- Creating a root signature");
        let root_sig = device.build_root_sig().build().unwrap();

        println!("- Creating a render pass");
        let pass = {
            let mut builder = device.build_render_pass();
            builder.end();
            builder.build().unwrap()
        };

        println!("- Creating a render target table");
        let rtt = device
            .build_render_target_table()
            .extents(&[256, 256])
            .build()
            .unwrap();

        println!("- Creating a pipeline");
        let pipeline = {
            let mut builder = device.build_render_pipeline();
            builder
                .vertex_shader(&library_vert, "main")
                .fragment_shader(&library_frag, "main")
                .root_sig(&root_sig)
                .render_pass(&pass, 0);
            builder.rasterize();
            builder.build().unwrap()
        };

        println!("- Creating a command buffer");
        let mut buffer = queue.new_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e = buffer.encode_render(&rtt);
            e.bind_pipeline(&pipeline);
            e.set_viewports(
                0,
                &[
                    gfx::Viewport {
                        x: 0.0,
                        y: 0.0,
                        width: 256.0,
                        height: 256.0,
                        min_depth: 0.0,
                        max_depth: 1.0,
                    },
                ],
            );
            e.draw(0..4, 0..1);
        }

        println!("- Installing a completion handler");
        let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer);

        println!("- Commiting the command buffer");
        buffer.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        awaiter.wait_until_completed();
    });
}
