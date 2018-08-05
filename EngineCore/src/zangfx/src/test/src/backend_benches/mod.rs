//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Benchmarks for ZanGFX implementations.
use crate::utils;
use zangfx_base as gfx;

pub trait BenchDriver {
    fn choose_device(&self, runner: &mut dyn FnMut(&gfx::DeviceRef));

    fn choose_compute_queue(&self, runner: &mut dyn FnMut(&gfx::DeviceRef, gfx::QueueFamily)) {
        self.choose_device(&mut |device| {
            for (i, qf) in device.caps().queue_families().iter().enumerate() {
                if qf.caps.intersects(gfx::limits::QueueFamilyCaps::Compute) {
                    println!("[Queue Family #{}]", i);
                    runner(device, i as _);
                    break;
                }
            }
        })
    }

    fn choose_render_queue(&self, runner: &mut dyn FnMut(&gfx::DeviceRef, gfx::QueueFamily)) {
        self.choose_device(&mut |device| {
            for (i, qf) in device.caps().queue_families().iter().enumerate() {
                if qf.caps.intersects(gfx::limits::QueueFamilyCaps::Render) {
                    println!("[Queue Family #{}]", i);
                    runner(device, i as _);
                    break;
                }
            }
        })
    }

    fn choose_copy_queue(&self, runner: &mut dyn FnMut(&gfx::DeviceRef, gfx::QueueFamily)) {
        self.choose_device(&mut |device| {
            for (i, qf) in device.caps().queue_families().iter().enumerate() {
                if qf.caps.intersects(gfx::limits::QueueFamilyCaps::Copy) {
                    println!("[Queue Family #{}]", i);
                    runner(device, i as _);
                    break;
                }
            }
        })
    }
}

/// Generates benchmark cases given a bench driver.
#[macro_export]
macro_rules! zangfx_generate_backend_benches {
    ($driver:expr) => {
        zangfx_bench_single! { cb_throughput_100, $driver }

        zangfx_bench_single! { cmds_dispatch_10000_throughput, $driver }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! zangfx_bench_single {
    ($(#[$m:meta])* $name:ident, $driver:expr) => {
        $(#[$m])*
        #[bench]
        fn $name(b: &mut $crate::test::Bencher) {
            $crate::backend_benches::$name($driver, b);
        }
    }
}

mod cmdbuffer;
pub use self::cmdbuffer::*;

mod cmds;
pub use self::cmds::*;
