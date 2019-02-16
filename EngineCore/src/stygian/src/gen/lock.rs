//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::DerefMut;
use parking_lot::Mutex;

/// Lockable object.
pub trait Lock {
    type Target;

    /// Acquire a lock, returning a smart reference to the inner object.
    ///
    /// The returned reference has to be boxed due to the language restriction.
    /// Hopefully, dynamic allocations could be optimized out by LLVM like
    /// this example: <https://godbolt.org/z/Bshdda>.
    fn lock<'a>(&'a mut self) -> Box<dyn DerefMut<Target = Self::Target> + 'a>;
}

impl<T> Lock for &'_ mut T {
    type Target = T;

    fn lock<'a>(&'a mut self) -> Box<dyn DerefMut<Target = Self::Target> + 'a> {
        Box::new(&mut **self)
    }
}

impl<T> Lock for &Mutex<T> {
    type Target = T;

    fn lock<'a>(&'a mut self) -> Box<dyn DerefMut<Target = Self::Target> + 'a> {
       Box::new((**self).lock())
    }
}
