//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use test::Bencher;
use super::{utils, BenchDriver};
use gfx::prelude::*;

static SPIRV_NULL: ::include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/compute_null.comp.spv"));

fn cb_throughput<T: BenchDriver>(driver: T, b: &mut Bencher, num_cbs: usize) {
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

        let mut pool = queue.new_cmd_pool().unwrap();

        let mut cb_ring: Vec<Box<FnMut()>> = (0..5).map(|_| Box::new(|| {}) as _).collect();

        b.iter(|| {
            device.autorelease_pool_scope_core(&mut |arp| {
                for i in 0..10 {
                    let cb_idx = i % cb_ring.len();
                    {
                        let await = &mut cb_ring[cb_idx];
                        await();
                    }

                    let mut awaiters: Vec<_> = (0..num_cbs)
                        .map(|_| {
                            let mut buffer = pool.begin_cmd_buffer().unwrap();
                            {
                                let e = buffer.encode_compute();
                                e.bind_pipeline(&pipeline);
                                e.dispatch(&[]);
                            }

                            let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer);
                            buffer.commit().unwrap();
                            awaiter
                        })
                        .collect();

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

        for mut await in cb_ring.drain(..) {
            await();
        }
    });
}

pub fn cb_throughput_100<T: BenchDriver>(driver: T, b: &mut Bencher) {
    cb_throughput(driver, b, 10);
}

pub fn cb_throughput_200<T: BenchDriver>(driver: T, b: &mut Bencher) {
    cb_throughput(driver, b, 20);
}

pub fn cb_throughput_400<T: BenchDriver>(driver: T, b: &mut Bencher) {
    cb_throughput(driver, b, 40);
}
