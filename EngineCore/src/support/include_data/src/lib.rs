//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Contains a single macro named `include_data!` that includes
//! a file as `DataView` which can reinterpret the data in various ways using
//! the target endianness.
use std::slice::from_raw_parts;
use std::mem::size_of;

/// Includes a file as a reference to a `u32` slice, interpreted using the target
/// endianness.
#[macro_export]
macro_rules! include_data {
    ($file:expr) => {{
        const ENF: &$crate::AlignmentEnforcer<[u8]> = &$crate::AlignmentEnforcer(0, *include_bytes!($file));
        $crate::DataView(&ENF.1)
    }}
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DataView(pub &'static [u8]);

// This should make the storage at least 4-byte aligned... hahaha
#[doc(hidden)]
#[repr(C)]
pub struct AlignmentEnforcer<T: ?Sized>(pub u32, pub T);

impl DataView {
    pub unsafe fn unsafe_as_slice<T>(&self) -> &'static [T] {
        from_raw_parts(self.0.as_ptr() as *const T, self.0.len() / size_of::<T>())
    }

    pub fn as_u8_slice(&self) -> &'static [u8] {
        self.0
    }

    pub fn as_u16_slice(&self) -> &'static [u16] {
        unsafe { self.unsafe_as_slice::<u16>() }
    }

    pub fn as_u32_slice(&self) -> &'static [u32] {
        unsafe { self.unsafe_as_slice::<u32>() }
    }

    pub fn as_i8_slice(&self) -> &'static [i8] {
        unsafe { self.unsafe_as_slice::<i8>() }
    }

    pub fn as_i16_slice(&self) -> &'static [i16] {
        unsafe { self.unsafe_as_slice::<i16>() }
    }

    pub fn as_i32_slice(&self) -> &'static [i32] {
        unsafe { self.unsafe_as_slice::<i32>() }
    }
}
