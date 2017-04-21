//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

use std::marker::PhantomData;
use std::ops::{Deref, Drop};
use std::ptr::null_mut;

use super::ComPtr;
use super::comptr::{ComInterface, AsComPtr};

/// Implicitly owned immutable COM pointer.
#[derive(Debug)]
pub struct UnownedComPtr<'a, T: ComInterface + 'a> {
    comptr: ComPtr<T>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T: ComInterface + 'a> UnownedComPtr<'a, T> {
    pub fn null() -> Self {
        UnownedComPtr {
            comptr: ComPtr::null(),
            phantom: PhantomData,
        }
    }

    pub fn from_ref(x: &'a T) -> Self where T : AsComPtr<T> {
        let mut cp = ComPtr::<T>::new();
        *cp.as_mut_ptr() = x as *const T as *mut T;
        UnownedComPtr {
            comptr: cp,
            phantom: PhantomData,
        }
    }

    pub fn from_comptr(x: &'a ComPtr<T>) -> Self where T : AsComPtr<T> {
        let mut cp = ComPtr::<T>::new();
        *cp.as_mut_ptr() = x.as_ptr::<T>();
        UnownedComPtr {
            comptr: cp,
            phantom: PhantomData,
        }
    }
}

impl<'a, T: ComInterface + 'a> Deref for UnownedComPtr<'a, T> {
    type Target = ComPtr<T>;

    fn deref(&self) -> &Self::Target {
        &self.comptr
    }
}

impl<'a, T: ComInterface + 'a> Drop for UnownedComPtr<'a, T> {
    fn drop(&mut self) {
        // prevent releasing the object
        *self.comptr.as_mut_ptr() = null_mut();
    }
}
