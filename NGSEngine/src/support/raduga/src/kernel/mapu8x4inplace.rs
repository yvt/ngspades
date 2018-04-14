//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {ScalarMode, SimdMode};

/// Kernels that apply a function on an array of `[u8; 4]s`.
pub trait MapU8x4InplaceKernel {
    fn apply<M: SimdMode>(&self, x: [M::U8; 4]) -> [M::U8; 4];
}

/// Extension trait for `MapU8x4InplaceKernel`.
pub trait MapU8x4InplaceKernelExt: MapU8x4InplaceKernel {
    /// Run a mapping kernel on a given slice.
    fn dispatch(&self, slice: &mut [u8]) {
        // TODO: Try SIMD mode
        self.dispatch_scalar(slice)
    }

    #[doc(hidden)]
    fn dispatch_scalar(&self, slice: &mut [u8]) {
        let mut i = 0;
        while i + 3 < slice.len() {
            let input = [slice[i], slice[i + 1], slice[i + 2], slice[i + 3]];
            let output = self.apply::<ScalarMode>(input);
            slice[i] = output[0];
            slice[i + 1] = output[1];
            slice[i + 2] = output[2];
            slice[i + 3] = output[3];
            i += 4;
        }
    }
}

impl<T: MapU8x4InplaceKernel + ?Sized> MapU8x4InplaceKernelExt for T {}
