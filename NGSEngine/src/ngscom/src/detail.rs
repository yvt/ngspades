//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/*
 * Helper functions for macros
 */

use std::mem;
use super::StaticOffset;

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
pub unsafe fn resolve_parent_object<'a, TOffset, TInterface, TClass>(this: *mut TInterface)
                                                                     -> &'a TClass
    where TOffset: StaticOffset
{
    let addr: isize = mem::transmute(this);
    mem::transmute(addr + TOffset::offset())
}

pub use std::sync::atomic::{AtomicIsize, Ordering, fence};
