//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides container types that provide implementations for `PartialEq` and
//! `Eq` based on a referential equality.
// (copied from `ngsgfx/src/common/src/refeq.rs`)
#![feature(unsize, coerce_unsized)]
use std::hash::Hasher;
use std::marker::Unsize;
use std::ops::{Deref, CoerceUnsized};
use std::sync::Arc;
use std::ptr;

/// Compute a hash value based on the referential equality on references.
///
/// This would break if Rust had a moving garbage collector.
pub fn ref_hash<T: ?Sized, H: Hasher>(value: &T, state: &mut H) {
    state.write_usize(unsafe { ::std::mem::transmute_copy(&(value as *const T)) });
}

/// `Box` wrapper that provides a referential equality.
#[derive(Debug)]
pub struct RefEqBox<T: ?Sized>(Box<T>);

impl<T: ?Sized> PartialEq for RefEqBox<T> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(&*self.0, &*other.0)
    }
}
impl<T: ?Sized> Eq for RefEqBox<T> {}
impl<T: ?Sized> ::std::hash::Hash for RefEqBox<T> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        ref_hash(&*self.0, state);
    }
}

impl<T> RefEqBox<T> {
    pub fn new(data: T) -> Self {
        RefEqBox(Box::new(data))
    }
}

impl<T: Clone> Clone for RefEqBox<T> {
    fn clone(&self) -> Self {
        RefEqBox(self.0.clone())
    }
}

impl<T: ?Sized> Deref for RefEqBox<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T: ?Sized> ::std::ops::DerefMut for RefEqBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

/// `Arc` wrapper that provides a referential equality.
#[derive(Debug)]
pub struct RefEqArc<T: ?Sized>(Arc<T>);

impl<T: ?Sized> PartialEq for RefEqArc<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: ?Sized> Eq for RefEqArc<T> {}
impl<T: ?Sized> ::std::hash::Hash for RefEqArc<T> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        ref_hash(&*self.0, state);
    }
}

impl<T> RefEqArc<T> {
    pub fn new(data: T) -> Self {
        RefEqArc(Arc::new(data))
    }

    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        match Arc::try_unwrap(this.0) {
            Ok(obj) => Ok(obj),
            Err(arc) => Err(RefEqArc(arc)),
        }
    }
}

impl<T: ?Sized> RefEqArc<T> {
    pub fn from_arc(arc: Arc<T>) -> Self {
        RefEqArc(arc)
    }

    pub fn into_arc(this: Self) -> Arc<T> {
        this.0
    }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<RefEqArc<U>> for RefEqArc<T> {}

impl<T: ?Sized> Clone for RefEqArc<T> {
    fn clone(&self) -> Self {
        RefEqArc(self.0.clone())
    }
}

impl<T: ?Sized> Deref for RefEqArc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
