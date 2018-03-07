//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Tests for ZanGFX implementations.
use gfx;

pub trait TestDriver {
    fn for_each_device(&self, runner: &mut FnMut(&gfx::Device));

    /// Retrieve if the backend is based on a safe implementation, i.e., does
    /// not cause an undefined behavior on invalid usages.
    fn is_safe(&self) -> bool {
        false
    }

    fn for_each_compute_queue(&self, runner: &mut FnMut(&gfx::Device, gfx::QueueFamily)) {
        self.for_each_device(&mut |device| {
            for (i, qf) in device.caps().queue_families().iter().enumerate() {
                if qf.caps.intersects(gfx::limits::QueueFamilyCaps::Compute) {
                    println!("[Queue Family #{}]", i);
                    runner(device, i as _);
                }
            }
        })
    }

    fn for_each_render_queue(&self, runner: &mut FnMut(&gfx::Device, gfx::QueueFamily)) {
        self.for_each_device(&mut |device| {
            for (i, qf) in device.caps().queue_families().iter().enumerate() {
                if qf.caps.intersects(gfx::limits::QueueFamilyCaps::Render) {
                    println!("[Queue Family #{}]", i);
                    runner(device, i as _);
                }
            }
        })
    }

    fn for_each_copy_queue(&self, runner: &mut FnMut(&gfx::Device, gfx::QueueFamily)) {
        self.for_each_device(&mut |device| {
            for (i, qf) in device.caps().queue_families().iter().enumerate() {
                if qf.caps.intersects(gfx::limits::QueueFamilyCaps::Copy) {
                    println!("[Queue Family #{}]", i);
                    runner(device, i as _);
                }
            }
        })
    }
}

/// Generates test cases given a test driver.
#[macro_export]
macro_rules! zangfx_generate_backend_tests {
    ($driver:expr) => {
        zangfx_test_single! { create_device, $driver }

        zangfx_test_single! { arg_table_sig_create_image, $driver }
        zangfx_test_single! { arg_table_sig_create_buffer, $driver }
        zangfx_test_single! { arg_table_sig_create_sampler, $driver }
        zangfx_test_single! { arg_table_image, $driver }
        zangfx_test_single! { arg_table_buffer, $driver }
        zangfx_test_single! { arg_table_sampler, $driver }

        zangfx_test_single! { cmdqueue_create, $driver }
        zangfx_test_single! { #[should_panic] cmdqueue_create_fail_missing_queue_family, $driver }
        zangfx_test_single! { cmdqueue_create_buffer, $driver }
        zangfx_test_single! { cmdqueue_create_encoder, $driver }
        zangfx_test_single! { cmdqueue_buffer_noop_completes, $driver }
        zangfx_test_single! { cmdqueue_buffer_noop_completes_dropped_soon, $driver }
        zangfx_test_single! { cmdqueue_buffer_noop_multiple_completes, $driver }
        zangfx_test_single! { cmdqueue_buffer_fence_update_wait_completes, $driver }

        zangfx_test_single! { heap_dynamic_create, $driver }
        zangfx_test_single! { #[should_panic] heap_dynamic_create_fail_zero_size, $driver }
        zangfx_test_single! { #[should_panic] heap_dynamic_create_fail_missing_memory_type, $driver }
        zangfx_test_single! { heap_dynamic_alloc_buffer, $driver }
        zangfx_test_single! { heap_dynamic_alloc_image, $driver }
        zangfx_test_single! { #[should_panic] heap_dedicated_create_fail_zero_size, $driver }
        zangfx_test_single! { #[should_panic] heap_dedicated_create_fail_missing_memory_type, $driver }
        zangfx_test_single! { heap_dedicated_alloc_buffer, $driver }
        zangfx_test_single! { heap_dedicated_alloc_image, $driver }

        zangfx_test_single! { image_all_formats, $driver }
        zangfx_test_single! { image_all_types, $driver }

        zangfx_test_single! { copy_fill_buffer, $driver }
        zangfx_test_single! { copy_copy_buffer, $driver }

        zangfx_test_single! { compute_null, $driver }
        zangfx_test_single! { compute_conv1, $driver }

        zangfx_test_single! { render_null, $driver }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! zangfx_test_single {
    ($(#[$m:meta])* $name:ident, $driver:expr) => {
        $(#[$m])*
        #[test]
        fn $name() {
            $crate::backend_tests::$name($driver);
        }
    }
}

pub fn create_device<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |_| {});
}

mod utils;

mod arg_table;
pub use self::arg_table::*;

mod cmdqueue;
pub use self::cmdqueue::*;

mod heap;
pub use self::heap::*;

mod image;
pub use self::image::*;

mod copy;
pub use self::copy::*;

mod compute_null;
pub use self::compute_null::*;

mod compute_conv1;
pub use self::compute_conv1::*;

mod render_null;
pub use self::render_null::*;
