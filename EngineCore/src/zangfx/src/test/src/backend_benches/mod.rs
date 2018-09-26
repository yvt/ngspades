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

/// A wrapper around `test::Bencher`.
///
/// Our crate do not have a direct access to the `test` crate even with
/// `#![feature(test)]` enabled. Therefore, callers of our benchmark functions
/// must wrap a supplied `test::Bencher` with a newtype and implement this
/// `Bencher` trait on the newtype.
/// (Backend implementor usually do not have to do this because the
/// [`zangfx_generate_backend_benches`] macro automatically handle that.)
/// This seems to have started at some point during the Rust 2018 transition.
pub trait Bencher {
    fn iter<T>(&mut self, f: impl FnMut() -> T);
}

/// Generates benchmark cases given a bench driver.
#[macro_export]
macro_rules! zangfx_generate_backend_benches {
    ($driver:expr) => {
        $crate::zangfx_bench_single! { cb_throughput_100, $driver }

        $crate::zangfx_bench_single! { cmds_dispatch_10000_throughput, $driver }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! zangfx_bench_single {
    ($(#[$m:meta])* $name:ident, $driver:expr) => {
        $(#[$m])*
        #[bench]
        fn $name(b: &mut test::Bencher) {
            struct Bencher<'a>(&'a mut test::Bencher);
            impl<'a> $crate::backend_benches::Bencher for Bencher<'a> {
                fn iter<T>(&mut self, f: impl FnMut() -> T) {
                    self.0.iter(f)
                }
            }
            $crate::backend_benches::$name($driver, &mut Bencher(b));
        }
    }
}

mod cmdbuffer;
pub use self::cmdbuffer::*;

mod cmds;
pub use self::cmds::*;
