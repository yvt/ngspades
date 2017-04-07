
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
pub unsafe fn delete_obj_raw<T>(x: *mut T) {
    Box::from_raw(x);
}

#[doc(hidden)]
pub unsafe fn resolve_parent_object<'a, TOffset, TInterface, TClass>(this: *mut TInterface) -> *mut TClass
  where TOffset : StaticOffset {
    let addr: isize = mem::transmute(this);
    mem::transmute(addr + TOffset::offset())
}

pub use std::sync::atomic::{AtomicIsize, Ordering, fence};
