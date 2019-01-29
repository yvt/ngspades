//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! `Arc` that can be converted into a type similar to `Box` and the other way
//! around without a runtime overhead of dynamic allocations.
//!
//! The standard library provides a reference counted container named `Arc`.
//! Since it does not provide a mutual exclusion facility by itself, the
//! inner value can be moved out only if there is no other references to it.
//! This can be done by the function `Arc::try_unwrap`, which basically
//! copies the value and releases the space previously occupied by the value.
//! Therefore, this involves a memory free operation and its overhead can be
//! problematic if such a conversion has to be done frequently, and is
//! obviously redundant especially if the value is going to be converted back
//! to `Arc` later.
//!
//! `Arc::get_mut` returns a mutable reference to the inner value, but the
//! returned reference's only lives as long as the reference to the parent
//! `Arc` does. Because of this, `Arc::get_mut` has to be called every time the
//! value is required. `Arc::get_mut` imposes an overhead of multiple atomic
//! operations, which we want to avoid especially if we can be sure that there
//! will not be other references to the value for a extended period of time.
//!
//! `BArc` addresses this problem by providing a method named `try_into_box`,
//! which returns a `BArcBox` only if there are no strong or weak references to
//! the inner value. `BArcBox` can be converted back to `BArc` anytime by calling
//! `into_arc`, without constructing `BArc` again.
//!
//! Nomenclature
//! ------------
//!
//! `BArc` stands for `Box` convertible `Arc`.
use std::cell::UnsafeCell;
use std::sync::{Arc, Weak};
use std::{hash, ops};

/// `Arc` that can be converted into a type named `BArcBox` that works similar;y
/// to `Box` and the other way around without a runtime overhead of dynamic
/// allocations.
#[derive(Debug)]
pub struct BArc<T: ?Sized>(Arc<UnsafeCell<T>>);

#[derive(Debug)]
pub struct BWeak<T: ?Sized>(Weak<UnsafeCell<T>>);

/// Unique reference to `Arc`'s inner value.
#[derive(Debug)]
pub struct BArcBox<T: ?Sized>(Arc<UnsafeCell<T>>);

unsafe impl<T: ?Sized + Sync + Send> Send for BArc<T> {}
unsafe impl<T: ?Sized + Sync + Send> Sync for BArc<T> {}

unsafe impl<T: ?Sized + Sync + Send> Send for BWeak<T> {}
unsafe impl<T: ?Sized + Sync + Send> Sync for BWeak<T> {}

unsafe impl<T: ?Sized + Send> Send for BArcBox<T> {}
unsafe impl<T: ?Sized + Sync> Sync for BArcBox<T> {}

impl<T> BArc<T> {
    pub fn new(x: T) -> Self {
        BArc(Arc::new(UnsafeCell::new(x)))
    }

    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        match Arc::try_unwrap(this.0) {
            Ok(cell) => Ok(cell.into_inner()),
            Err(arc) => Err(BArc(arc)),
        }
    }

    pub fn into_raw(this: Self) -> *const T {
        unsafe { &*Arc::into_raw(this.0) }.get()
    }

    pub unsafe fn from_raw(ptr: *const T) -> Self {
        let usc = ptr as *const UnsafeCell<T>;
        BArc(Arc::from_raw(usc))
    }

    pub fn downgrade(this: &Self) -> BWeak<T> {
        BWeak(Arc::downgrade(&this.0))
    }

    pub fn strong_count(this: &Self) -> usize {
        Arc::strong_count(&this.0)
    }

    pub fn weak_count(this: &Self) -> usize {
        Arc::weak_count(&this.0)
    }

    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        Arc::ptr_eq(&this.0, &other.0)
    }

    pub fn try_into_box(mut this: Self) -> Result<BArcBox<T>, Self> {
        if Arc::get_mut(&mut this.0).is_some() {
            Ok(BArcBox(this.0))
        } else {
            Err(this)
        }
    }
}

impl<T: ?Sized + PartialEq> PartialEq for BArc<T> {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: ?Sized + Eq> Eq for BArc<T> {}

impl<T: ?Sized + hash::Hash> hash::Hash for BArc<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        hash::Hash::hash(&**self, state)
    }
}

impl<T: ?Sized> ops::Deref for BArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.get() }
    }
}

impl<T: ?Sized> Clone for BArc<T> {
    fn clone(&self) -> Self {
        BArc(self.0.clone())
    }
}

unsafe impl<T> atom2::PtrSized for BArc<T> {
    type Value = T;

    fn into_raw(this: Self) -> *const Self::Value {
        BArc::into_raw(this)
    }
    unsafe fn from_raw(ptr: *const Self::Value) -> Self {
        BArc::from_raw(ptr)
    }
}
unsafe impl<T> atom2::RcLike for BArc<T> {}

impl<T> atom2::AsRawPtr<T> for BArc<T> {
    fn as_raw_ptr(&self) -> *const T {
        &**self as *const _
    }
}

impl<T: ?Sized> Clone for BWeak<T> {
    fn clone(&self) -> Self {
        BWeak(self.0.clone())
    }
}

impl<T> BWeak<T> {
    pub fn new() -> Self {
        BWeak(Weak::new())
    }
}

impl<T: ?Sized> BWeak<T> {
    pub fn upgrade(&self) -> Option<BArc<T>> {
        self.0.upgrade().map(BArc)
    }
}

impl<T> BArcBox<T> {
    pub fn new(x: T) -> Self {
        BArcBox(Arc::new(UnsafeCell::new(x)))
    }

    pub fn into_arc(this: Self) -> BArc<T> {
        BArc(this.0)
    }
}

impl<T: ?Sized> ops::Deref for BArcBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.get() }
    }
}

impl<T: ?Sized> ops::DerefMut for BArcBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.get() }
    }
}
