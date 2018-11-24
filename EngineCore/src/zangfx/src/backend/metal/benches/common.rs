//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![warn(rust_2018_idioms)]
#![feature(test)]
// Despite the compiler warning, we still need this for benchmarking
#![allow(rust_2018_idioms)]
extern crate test;

use std::sync::Arc;

use zangfx_base::prelude::*;
use zangfx_test::zangfx_generate_backend_benches;

struct BenchDriver;

impl zangfx_test::backend_benches::BenchDriver for BenchDriver {
    fn choose_device(&self, runner: &mut dyn FnMut(&zangfx_base::DeviceRef)) {
        let device = unsafe { zangfx_metal::device::Device::new_system_default().unwrap() };
        let device: zangfx_base::DeviceRef = Arc::new(device);
        device.autorelease_pool_scope(|_| {
            runner(&device);
        });
    }
}

zangfx_generate_backend_benches!(BenchDriver);
