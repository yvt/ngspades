//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! A faster, drop-in replacaement for the [`objc`] crate's `msg_send!` and `sel!`.
//!
//! [`objc`]: ../objc/index.html
//!
//! # Benchmarks
//!
//! The optimized `msg_send!` that caches `Sel`:
//!
//! ```text
//! running 1 test
//! test msg_send_1000 ... bench:      36,937 ns/iter (+/- 4,701)
//! ```
//!
//! The original `msg_send!`:
//!
//! ```text
//! running 1 test
//! test msg_send_1000 ... bench:     125,287 ns/iter (+/- 172,079)
//! ```
//!
#![feature(intrinsics)]

extern crate objc;

// Just reuse `objc`'s `msg_send!`
pub use objc::msg_send;

use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

/// Registers a selector and caches the result for faster further accesses.
#[macro_export]
macro_rules! sel {
    ($name:ident) => ({
        static CACHE: $crate::SelCache = $crate::SEL_CACHE_INIT;
        $crate::register_sel(concat!(stringify!($name), '\0'), &CACHE)
    });
    ($($name:ident :)+) => ({
        static CACHE: $crate::SelCache = $crate::SEL_CACHE_INIT;
        $crate::register_sel(concat!($(stringify!($name), ':'),+, '\0'), &CACHE)
    });
}

#[doc(hidden)]
pub struct SelCache(AtomicUsize);

#[doc(hidden)]
pub const SEL_CACHE_INIT: SelCache = SelCache(ATOMIC_USIZE_INIT);

extern "rust-intrinsic" {
    fn unlikely(b: bool) -> bool;
}

#[inline(always)]
#[doc(hidden)]
pub fn register_sel(name_with_nul: &str, cache: &SelCache) -> objc::runtime::Sel {
    use std::mem::transmute;
    let mut sel_ptr = cache.0.load(Ordering::Relaxed);
    if unsafe { unlikely(sel_ptr == 0) } {
        let ptr = name_with_nul.as_ptr() as *const _;
        sel_ptr = unsafe { transmute(objc::runtime::sel_registerName(ptr)) };
        cache.0.store(sel_ptr, Ordering::Relaxed);
    }
    unsafe { transmute(sel_ptr) }
}
