//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{utils, BenchDriver, Bencher};
use include_data::include_data;
use zangfx_base::prelude::*;

static SPIRV_NULL: ::include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/compute_null.comp.spv"));

/// Issues a command buffer containing 10000 compute dispatches.
pub fn cmds_dispatch_10000_throughput<T: BenchDriver>(driver: T, b: &mut impl Bencher) {
    driver.choose_compute_queue(&mut |device, qf| {
        let queue = device.build_cmd_queue().queue_family(qf).build().unwrap();
        let library = device.new_library(SPIRV_NULL.as_u32_slice()).unwrap();
        let root_sig = device.build_root_sig().build().unwrap();

        let pipeline = device
            .build_compute_pipeline()
            .compute_shader(&library, "main")
            .root_sig(&root_sig)
            .build()
            .unwrap();

        b.iter(|| {
            device.autorelease_pool_scope_core(&mut |_| {
                let mut buffer = queue.new_cmd_buffer().unwrap();
                {
                    let e = buffer.encode_compute();
                    e.bind_pipeline(&pipeline);
                    for _ in 0..10000 {
                        e.dispatch(&[]);
                    }
                }

                let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer);
                buffer.commit().unwrap();
                queue.flush();
                awaiter.wait_until_completed();
            });
        });
    });
}
