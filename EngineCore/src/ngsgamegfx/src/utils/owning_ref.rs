//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Asserts address stability on external types (on which we can't `impl`
//! `StableAddress` due to coherency rules) to enable the uses of
//! [`owning_ref::OwningRef`] and [`owning_ref::OwningRefMut`].
use owning_ref::StableAddress;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssertStableAddress<T>(pub T);

impl<T: Deref> Deref for AssertStableAddress<T> {
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: DerefMut> DerefMut for AssertStableAddress<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

use atomic_refcell::{AtomicRef, AtomicRefMut};

unsafe impl<T> StableAddress for AssertStableAddress<AtomicRef<'_, T>> {}
unsafe impl<T> StableAddress for AssertStableAddress<AtomicRefMut<'_, T>> {}
