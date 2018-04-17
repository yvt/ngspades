//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate zangfx_base as base;
extern crate zangfx_metal as backend;
#[macro_use]
extern crate zangfx_test;

use base::prelude::*;

struct TestDriver;

impl zangfx_test::backend_tests::TestDriver for TestDriver {
    fn for_each_device(&self, runner: &mut FnMut(&base::device::Device)) {
        let device = unsafe { backend::device::Device::new_system_default().unwrap() };
        device.autorelease_pool_scope(|_| {
            runner(&device);
        });
    }
}

zangfx_generate_backend_tests!(TestDriver);
