//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{utils, TestDriver};
use include_data::include_data;
use zangfx_base::prelude::*;

static SPIRV_NULL: ::include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/compute_null.comp.spv"));

pub fn compute_null<T: TestDriver>(driver: T) {
    driver.for_each_compute_queue(&mut |device, qf| {
        println!("- Creating a command queue");
        let queue = device.build_cmd_queue().queue_family(qf).build().unwrap();

        println!("- Creating a library");
        let library = device.new_library(SPIRV_NULL.as_u32_slice()).unwrap();

        println!("- Creating a root signature");
        let root_sig = device.build_root_sig().build().unwrap();

        println!("- Creating a pipeline");
        let pipeline = device
            .build_compute_pipeline()
            .compute_shader(&library, "main")
            .root_sig(&root_sig)
            .build()
            .unwrap();

        println!("- Creating a command buffer");
        let mut buffer = queue.new_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e = buffer.encode_compute();
            e.bind_pipeline(&pipeline);
            e.dispatch(&[]);
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
