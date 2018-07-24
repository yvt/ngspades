//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{utils, TestDriver};
use include_data::include_data;
use ngsenumflags::flags;
use zangfx_base as gfx;
use zangfx_base::prelude::*;

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
            builder.target(0).set_format(<u8>::as_rgba_norm());
            builder.subpass_color_targets(&[Some(0)]);
            builder.build().unwrap()
        };

        println!("- Creating a render target");
        let image = device
            .build_image()
            .extents(&[256, 256])
            .format(<u8>::as_rgba_norm())
            .usage(flags![gfx::ImageUsage::{Render}])
            .build()
            .unwrap();

        println!("- Computing the memory requirements for the render target");
        let valid_memory_types = image.get_memory_req().unwrap().memory_types;
        let memory_type = utils::choose_memory_type(
            device,
            valid_memory_types,
            flags![gfx::MemoryTypeCaps::{DeviceLocal}],
            flags![gfx::MemoryTypeCaps::{DeviceLocal}],
        );
        println!("  Memory Type = {}", memory_type);

        println!("- Allocating memory");
        let heap = device.global_heap(memory_type);
        assert!(heap.bind((&image).into()).unwrap());

        println!("- Creating a render target table");
        let rtt = {
            let mut builder = device.build_render_target_table();
            builder.target(0, &image);
            builder
                .render_pass(&pass)
                .extents(&[256, 256])
                .build()
                .unwrap()
        };

        println!("- Creating a pipeline");
        let pipeline = {
            let mut builder = device.build_render_pipeline();
            builder
                .vertex_shader(&library_vert, "main")
                .fragment_shader(&library_frag, "main")
                .root_sig(&root_sig)
                .topology(gfx::PrimitiveTopology::Triangles)
                .render_pass(&pass, 0);
            {
                let raster = builder.rasterize();
                raster
                    .color_target(0)
                    .set_write_mask(flags![gfx::ColorChannel::{}]);
            }
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
                &[gfx::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: 256.0,
                    height: 256.0,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
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
