//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate zangfx_base as base;
#[macro_use]
extern crate zangfx_test;
extern crate zangfx_vulkan as backend;

use base::prelude::*;

struct TestDriver;

impl zangfx_test::backend_tests::TestDriver for TestDriver {
    fn for_each_device(&self, _runner: &mut FnMut(&base::device::Device)) {
        unimplemented!();
    }
}

zangfx_generate_backend_tests!(TestDriver);
