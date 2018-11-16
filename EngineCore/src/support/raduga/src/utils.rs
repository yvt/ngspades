//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use crate::{Packed, SimdMode};

/// Extensions methods for slices.
pub trait SliceExt<M: SimdMode> {
    type PackedElement: Packed<Mode = M>;

    /// Load multiple values from non-contiguous memory locations inside a slice.
    ///
    /// See [`Packed::gather32_unchecked`] for details.
    ///
    /// [`Packed::gather32_unchecked`]: Packed::gather32_unchecked
    unsafe fn gather32_unchecked(&self, offset: M::U32, scale: u8) -> Self::PackedElement;
}

impl<M: SimdMode> SliceExt<M> for [u8] {
    type PackedElement = M::U8;

    unsafe fn gather32_unchecked(&self, offset: M::U32, scale: u8) -> Self::PackedElement {
        M::U8::gather32_unchecked(self, offset, scale)
    }
}

impl<M: SimdMode> SliceExt<M> for [u16] {
    type PackedElement = M::U16;

    unsafe fn gather32_unchecked(&self, offset: M::U32, scale: u8) -> Self::PackedElement {
        M::U16::gather32_unchecked(self, offset, scale)
    }
}

impl<M: SimdMode> SliceExt<M> for [u32] {
    type PackedElement = M::U32;

    unsafe fn gather32_unchecked(&self, offset: M::U32, scale: u8) -> Self::PackedElement {
        M::U32::gather32_unchecked(self, offset, scale)
    }
}

impl<M: SimdMode> SliceExt<M> for [i16] {
    type PackedElement = M::I16;

    unsafe fn gather32_unchecked(&self, offset: M::U32, scale: u8) -> Self::PackedElement {
        M::I16::gather32_unchecked(self, offset, scale)
    }
}
