//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Reimplementation of the [atom] library with specialized and extended features.
//!
//! [atom]: https://crates.io/crates/atom
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::marker::PhantomData;
use std::{ptr, mem, fmt};

pub trait RcLike: Sized + Clone {
    type Value;

    fn into_raw(this: Self) -> *const Self::Value;
    unsafe fn from_raw(ptr: *const Self::Value) -> Self;
}

impl<T> RcLike for Arc<T> {
    type Value = T;

    fn into_raw(this: Self) -> *const Self::Value {
        Arc::into_raw(this)
    }
    unsafe fn from_raw(ptr: *const Self::Value) -> Self {
        Arc::from_raw(ptr)
    }
}

impl<T> RcLike for ::barc::BArc<T> {
    type Value = T;

    fn into_raw(this: Self) -> *const Self::Value {
        ::barc::BArc::into_raw(this)
    }
    unsafe fn from_raw(ptr: *const Self::Value) -> Self {
        ::barc::BArc::from_raw(ptr)
    }
}

pub struct AtomicArc<T: RcLike> {
    ptr: AtomicPtr<T::Value>,
    phantom: PhantomData<T>,
}

unsafe impl<T: RcLike + Sync> Sync for AtomicArc<T> {}
unsafe impl<T: RcLike + Send> Send for AtomicArc<T> {}

unsafe fn option_arc_from_raw<T: RcLike>(p: *const T::Value) -> Option<T> {
    if p.is_null() {
        None
    } else {
        Some(T::from_raw(p))
    }
}

fn option_arc_into_raw<T: RcLike>(x: Option<T>) -> *const T::Value {
    if let Some(x) = x {
        RcLike::into_raw(x)
    } else {
        ptr::null()
    }
}

/// An atomic `Option<Arc<T>>` storage that can be safely shared between threads.
impl<T: RcLike> AtomicArc<T> {
    pub fn empty() -> Self {
        Self {
            ptr: AtomicPtr::default(),
            phantom: PhantomData,
        }
    }

    pub fn new(x: Option<T>) -> Self {
        Self {
            ptr: AtomicPtr::new(option_arc_into_raw(x) as *mut T::Value),
            phantom: PhantomData,
        }
    }

    pub fn into_inner(mut self) -> Option<T> {
        let p = mem::replace(&mut self.ptr, AtomicPtr::default()).into_inner();

        // skip `drop`
        mem::forget(self);

        unsafe { option_arc_from_raw(p) }
    }

    pub fn as_ref(&mut self) -> Option<&T::Value> {
        let p = *self.ptr.get_mut();
        if p.is_null() {
            None
        } else {
            Some(unsafe { &*p })
        }
    }

    pub fn load(&mut self) -> Option<T> {
        let mut p = self.ptr.get_mut();
        if let Some(arc) = unsafe { option_arc_from_raw::<T>(*p) } {
            let ret = T::clone(&arc);
            *p = T::into_raw(arc) as *mut _;
            Some(ret)
        } else {
            None
        }
    }

    pub fn swap(&self, x: Option<T>, order: Ordering) -> Option<T> {
        let new_ptr = option_arc_into_raw(x);
        let old_ptr = self.ptr.swap(new_ptr as *mut T::Value, order);
        unsafe { option_arc_from_raw(old_ptr) }
    }

    pub fn store(&self, x: Option<T>, order: Ordering) {
        self.swap(x, order);
    }

    pub fn take(&self, order: Ordering) -> Option<T> {
        self.swap(None, order)
    }

    /// Stores a value into the storage if the current value is the same as the
    /// `current` value.
    ///
    /// Returns the previous value with `Ok(x)` if the value was updated.
    /// `Err(new)` otherwise.
    pub fn compare_and_swap<P: AsRawPtr<T::Value>>(
        &self,
        current: &P,
        new: Option<T>,
        order: Ordering,
    ) -> Result<Option<T>, Option<T>> {
        let new_ptr = option_arc_into_raw(new);
        let current_ptr = current.as_raw_ptr();
        let old_ptr = self.ptr.compare_and_swap(
            current_ptr as *mut T::Value,
            new_ptr as *mut T::Value,
            order,
        );
        if old_ptr == current_ptr as *mut T::Value {
            // Successful
            Ok(unsafe { option_arc_from_raw(old_ptr) })
        } else {
            // Failure
            Err(unsafe { option_arc_from_raw(new_ptr) })
        }
    }
}

impl<T: RcLike> fmt::Debug for AtomicArc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("AtomicArc").field(&self.ptr).finish()
    }
}

impl<T: RcLike> Drop for AtomicArc<T> {
    fn drop(&mut self) {
        self.take(Ordering::Relaxed);
    }
}

impl<T: RcLike> Default for AtomicArc<T> {
    fn default() -> Self {
        AtomicArc::empty()
    }
}

pub trait AsRawPtr<T> {
    fn as_raw_ptr(&self) -> *const T;
}

impl<'a, T> AsRawPtr<T> for *const T {
    fn as_raw_ptr(&self) -> *const T {
        *self
    }
}

impl<'a, T> AsRawPtr<T> for &'a T {
    fn as_raw_ptr(&self) -> *const T {
        *self as *const _
    }
}

impl<'a, T> AsRawPtr<T> for &'a mut T {
    fn as_raw_ptr(&self) -> *const T {
        *self as *const _
    }
}

impl<T> AsRawPtr<T> for Arc<T> {
    fn as_raw_ptr(&self) -> *const T {
        &**self as *const _
    }
}

impl<T> AsRawPtr<T> for ::barc::BArc<T> {
    fn as_raw_ptr(&self) -> *const T {
        &**self as *const _
    }
}

impl<T, S> AsRawPtr<T> for Option<S>
where
    S: AsRawPtr<T>,
{
    fn as_raw_ptr(&self) -> *const T {
        if let &Some(ref p) = self {
            p.as_raw_ptr()
        } else {
            ptr::null()
        }
    }
}
