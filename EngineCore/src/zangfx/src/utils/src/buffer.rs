//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use num_traits::ToPrimitive;
use pod::Pod;
use volatile_view::{prelude::*, Volatile};
use zangfx_base as base;

/// An extension trait for `Buffer`.
pub trait BufferUtils: base::Buffer {
    /// Get a volatile access view of bytes in the underlying storage of a
    /// buffer.
    ///
    /// # Panics
    ///
    /// Panics if `len()` does not fit in `usize`.
    ///
    /// # Valid Usage
    ///
    ///  - The buffer must be in the **Allocated** state.
    ///  - The buffer must be bound to a heap whose memory type is host-visible.
    ///
    /// # Examples
    ///
    ///     use zangfx_base::*;
    ///     use zangfx_utils::BufferUtils;
    ///     # fn test(buffer: &BufferRef) {
    ///     let view: &[_] = buffer.as_bytes_volatile();
    ///     for x in view {
    ///         x.store(0);
    ///     }
    ///     # }
    ///
    fn as_bytes_volatile(&self) -> &[Volatile<u8>] {
        let len = self.len().to_usize().expect("len overflow");
        unsafe { Volatile::slice_from_raw(self.as_ptr(), len) }
    }

    /// Get a volatile access view of values in the underlying storage of a
    /// buffer.
    ///
    /// Returns `None` if the storage is misaligned or its size does not
    /// perfectly fit in `[T]`.
    ///
    /// # Panics
    ///
    /// Panics if `len()` does not fit in `usize`.
    ///
    /// # Valid Usage
    ///
    ///  - The buffer must be in the **Allocated** state.
    ///  - The buffer must be bound to a heap whose memory type is host-visible.
    ///
    /// # Examples
    ///
    ///     use zangfx_base::*;
    ///     use zangfx_utils::BufferUtils;
    ///     # fn test(buffer: &BufferRef) {
    ///     let view: &[_] = buffer.as_volatile::<u32>().unwrap();
    ///     for x in view {
    ///         x.store(0);
    ///     }
    ///     # }
    ///
    fn as_volatile<T: Pod>(&self) -> Option<&[Volatile<T>]> {
        self.as_bytes_volatile().map_slice()
    }
}

impl<T: base::Buffer + ?Sized> BufferUtils for T {}
