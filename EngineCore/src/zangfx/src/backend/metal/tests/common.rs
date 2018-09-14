//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![warn(rust_2018_idioms)]

use std::sync::Arc;

use zangfx_base::prelude::*;
use zangfx_test::zangfx_generate_backend_tests;

struct TestDriver;

impl zangfx_test::backend_tests::TestDriver for TestDriver {
    fn for_each_device(&self, runner: &mut dyn FnMut(&zangfx_base::DeviceRef)) {
        let device = unsafe { zangfx_metal::device::Device::new_system_default().unwrap() };
        let device: zangfx_base::DeviceRef = Arc::new(device);
        device.autorelease_pool_scope(|_| {
            runner(&device);
        });
    }
}

zangfx_generate_backend_tests!(TestDriver);
