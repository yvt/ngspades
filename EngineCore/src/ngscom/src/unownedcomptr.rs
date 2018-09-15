//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::Deref;

use super::comptr::{AsComPtr, ComInterface};
use super::ComPtr;

/// Implicitly owned immutable COM pointer.
#[derive(Debug)]
pub struct UnownedComPtr<'a, T: ComInterface + 'a> {
    comptr: ManuallyDrop<ComPtr<T>>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T: ComInterface + 'a> UnownedComPtr<'a, T> {
    pub fn null() -> Self {
        UnownedComPtr {
            comptr: ManuallyDrop::new(ComPtr::null()),
            phantom: PhantomData,
        }
    }

    pub fn to_owned(&self) -> ComPtr<T> {
        ComPtr::clone(&self.comptr)
    }

    pub fn from_ref(x: &'a T) -> Self
    where
        T: AsComPtr<T>,
    {
        let mut cp = ComPtr::<T>::new();
        *cp.as_mut_ptr() = x as *const T as *mut T;
        UnownedComPtr {
            comptr: ManuallyDrop::new(cp),
            phantom: PhantomData,
        }
    }

    pub fn from_comptr(x: &'a ComPtr<T>) -> Self
    where
        T: AsComPtr<T>,
    {
        let mut cp = ComPtr::<T>::new();
        *cp.as_mut_ptr() = x.as_ptr::<T>();
        UnownedComPtr {
            comptr: ManuallyDrop::new(cp),
            phantom: PhantomData,
        }
    }
}

impl<'a, T: ComInterface + 'a> Deref for UnownedComPtr<'a, T> {
    type Target = ComPtr<T>;

    fn deref(&self) -> &Self::Target {
        &*self.comptr
    }
}

impl<'a, T: ComInterface + 'a> AsRef<ComPtr<T>> for UnownedComPtr<'a, T> {
    fn as_ref(&self) -> &ComPtr<T> {
        &*self.comptr
    }
}

impl<'a, T: ComInterface + 'a> From<&'a ComPtr<T>> for UnownedComPtr<'a, T>
where
    T: AsComPtr<T>,
{
    fn from(x: &'a ComPtr<T>) -> Self {
        Self::from_comptr(x)
    }
}
