//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(test)]
extern crate zangfx_base as base;
extern crate zangfx_metal as backend;
#[macro_use]
extern crate zangfx_test;

use std::sync::Arc;

use base::prelude::*;

struct BenchDriver;

impl zangfx_test::backend_benches::BenchDriver for BenchDriver {
    fn choose_device(&self, runner: &mut FnMut(&base::DeviceRef)) {
        let device = unsafe { backend::device::Device::new_system_default().unwrap() };
        let device: base::DeviceRef = Arc::new(device);
        device.autorelease_pool_scope(|_| {
            runner(&device);
        });
    }
}

zangfx_generate_backend_benches!(BenchDriver);
