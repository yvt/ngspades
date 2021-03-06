//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//
// This source code is based on com-rs. The original license text is shown below:
//
//     Copyright (c) 2016 com-rs developers
//
//     Licensed under the Apache License, Version 2.0
//     <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
//     license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
//     option. All files in the project carrying such notice may not be copied,
//     modified, or distributed except according to those terms.
//
use std::os::raw::c_void;

use super::detail::resolve_parent_object;
use super::{AsComPtr, HResult, StaticOffset, IID};

/// Base interface for all COM types.
///
/// None of the methods on this struct should be called directly,
/// use [`ComPtr`](struct.ComPtr.html) instead.
#[derive(Debug)]
#[repr(C)]
pub struct IUnknown {
    vtable: *const IUnknownVtbl,
}

#[allow(missing_debug_implementations)]
#[repr(C)]
#[doc(hidden)]
pub struct IUnknownVtbl {
    query_interface: extern "C" fn(*mut IUnknown, &IID, *mut *mut c_void) -> HResult,
    add_ref: extern "C" fn(*mut IUnknown) -> u32,
    release: extern "C" fn(*mut IUnknown) -> u32,
}

com_iid!(
    IID_IUNKNOWN = [
        0x00000000,
        0x0000,
        0x0000,
        [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
    ]
);

impl IUnknown {
    /// Retrieves pointers to the supported interfaces on an object.
    /// Use [`ComPtr::from`](struct.ComPtr.html#method.from) instead.
    pub fn query_interface(&self, iid: &IID, object: *mut *mut c_void) -> HResult {
        unsafe { ((*self.vtable).query_interface)(self as *const Self as *mut Self, iid, object) }
    }

    /// Increments the reference count for an interface on an object.
    /// Should never need to call this directly.
    pub fn add_ref(&self) -> u32 {
        unsafe { ((*self.vtable).add_ref)(self as *const Self as *mut Self) }
    }

    /// Decrements the reference count for an interface on an object.
    /// Should never need to call this directly.
    pub unsafe fn release(&self) -> u32 {
        ((*self.vtable).release)(self as *const Self as *mut Self)
    }

    pub fn from_vtable(vtable: *const IUnknownVtbl) -> Self {
        Self { vtable: vtable }
    }

    pub fn fill_vtable<T, S>() -> IUnknownVtbl
    where
        T: IUnknownTrait,
        S: StaticOffset,
    {
        IUnknownVtbl {
            query_interface: IUnknownThunk::query_interface::<T, S>,
            add_ref: IUnknownThunk::add_ref::<T, S>,
            release: IUnknownThunk::release::<T, S>,
        }
    }

    pub fn scan_iid(iid: &IID) -> bool {
        *iid == IID_IUNKNOWN
    }
}

struct IUnknownThunk();

impl IUnknownThunk {
    extern "C" fn query_interface<T: IUnknownTrait, S: StaticOffset>(
        this: *mut IUnknown,
        iid: &IID,
        object: *mut *mut c_void,
    ) -> HResult {
        unsafe { T::query_interface(resolve_parent_object::<S, IUnknown, T>(this), iid, object) }
    }
    extern "C" fn add_ref<T: IUnknownTrait, S: StaticOffset>(this: *mut IUnknown) -> u32 {
        unsafe { T::add_ref(resolve_parent_object::<S, IUnknown, T>(this)) }
    }
    extern "C" fn release<T: IUnknownTrait, S: StaticOffset>(this: *mut IUnknown) -> u32 {
        unsafe { T::release(resolve_parent_object::<S, IUnknown, T>(this)) }
    }
}

pub trait IUnknownTrait: Sync + Send {
    fn query_interface(&self, iid: &IID, object: *mut *mut c_void) -> HResult
    where
        Self: Sized;
    fn add_ref(&self) -> u32
    where
        Self: Sized;
    unsafe fn release(&self) -> u32
    where
        Self: Sized;
}

unsafe impl AsComPtr<IUnknown> for IUnknown {}

unsafe impl crate::ComInterface for IUnknown {
    #[doc(hidden)]
    type Vtable = IUnknownVtbl;
    #[doc(hidden)]
    type Trait = dyn IUnknownTrait;
    fn iid() -> crate::IID {
        IID_IUNKNOWN
    }
}
