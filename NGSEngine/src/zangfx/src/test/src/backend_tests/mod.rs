//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Tests for ZanGFX implementations.
use base;

pub trait TestDriver {
    fn for_each_device(&self, runner: &mut FnMut(&base::device::Device));

    /// Retrieve if the backend is based on a safe implementation, i.e., does
    /// not cause an undefined behavior on invalid usages.
    fn is_safe(&self) -> bool {
        false
    }
}

/// Generates test cases given a test driver.
#[macro_export]
macro_rules! zangfx_generate_backend_tests {
    ($driver:expr) => {
        zangfx_test_single! { create_device, $driver }
        zangfx_test_single! { cmdqueue_create, $driver }
        #[should_panic] zangfx_test_single! { cmdqueue_create_fail_missing_queue_family, $driver }
        zangfx_test_single! { cmdqueue_create_buffer, $driver }
        zangfx_test_single! { cmdqueue_buffer_noop_completes, $driver }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! zangfx_test_single {
    ($name:ident, $driver:expr) => {
        #[test]
        fn $name() {
            $crate::backend_tests::$name($driver);
        }
    }
}

pub fn create_device<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |_| {});
}

mod cmdqueue;
pub use self::cmdqueue::*;
