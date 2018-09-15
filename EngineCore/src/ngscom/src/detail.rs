//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Helper functions for macros

use super::StaticOffset;
use std::mem;

#[doc(hidden)]
pub fn new_obj_raw<T>(x: T) -> *mut T {
    Box::into_raw(Box::new(x))
}

#[doc(hidden)]
pub unsafe fn delete_obj_raw<T>(x: &T) {
    // at this point there's only one reference to `x` so it's safe to
    // transmute it to a mutable reference
    let ptr: *mut T = mem::transmute(x);
    Box::from_raw(ptr);
}

#[doc(hidden)]
pub unsafe fn resolve_parent_object<'a, TOffset, TInterface, TClass>(
    this: *mut TInterface,
) -> &'a TClass
where
    TOffset: StaticOffset,
{
    let addr: isize = mem::transmute(this);
    mem::transmute(addr + TOffset::offset())
}

/// Zero-sized type used to prohibit the on-stack construction of COM class header data.
#[doc(hidden)]
#[derive(Debug)]
pub struct ComClassHeader(());

impl ComClassHeader {
    pub unsafe fn new() -> Self {
        ComClassHeader(())
    }
}

#[doc(hidden)]
pub use std::sync::atomic::{fence, AtomicIsize, Ordering};

/// Enforces `Send` and `Sync` on class data.
#[doc(hidden)]
pub trait SyncAndSend: Sync + Send {}
