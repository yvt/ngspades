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

fn cb_throughput<T: BenchDriver>(driver: T, b: &mut impl Bencher, num_cbs: usize) {
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

        let mut cb_ring: Vec<Box<dyn FnMut()>> = (0..5).map(|_| Box::new(|| {}) as _).collect();

        b.iter(|| {
            device.autorelease_pool_scope_core(&mut |arp| {
                for i in 0..10 {
                    let cb_idx = i % cb_ring.len();
                    {
                        let wait_completion = &mut cb_ring[cb_idx];
                        wait_completion();
                    }

                    let mut awaiters: Vec<_> = (0..num_cbs)
                        .map(|_| {
                            let mut buffer = queue.new_cmd_buffer().unwrap();
                            {
                                let e = buffer.encode_compute();
                                e.bind_pipeline(&pipeline);
                                e.dispatch(&[]);
                            }

                            let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer);
                            buffer.commit().unwrap();
                            awaiter
                        }).collect();

                    queue.flush();
                    cb_ring[cb_idx] = Box::new(move || {
                        for awaiter in awaiters.drain(..) {
                            awaiter.wait_until_completed();
                        }
                    });

                    arp.drain();
                }
            });
        });

        for mut wait_completion in cb_ring.drain(..) {
            wait_completion();
        }
    });
}

pub fn cb_throughput_100<T: BenchDriver>(driver: T, b: &mut impl Bencher) {
    cb_throughput(driver, b, 10);
}
