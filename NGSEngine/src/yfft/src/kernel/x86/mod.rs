//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

//! Defines FFT kernels optimized for x86 and x86_64 systems.

use super::{Kernel, KernelCreationParams, KernelParams, KernelType, SliceAccessor};
use super::super::Num;
use super::utils;

mod bitreversal;
mod x86sse1radix2;
mod x86sse1radix4;
mod x86sse2;
mod x86sse3;

pub fn new_x86_kernel<T>(cparams: &KernelCreationParams) -> Option<Box<Kernel<T>>>
    where T : Num {
    x86sse3::new_x86_sse3_kernel(cparams)
        .or_else(|| x86sse2::new_x86_sse2_kernel(cparams))
        .or_else(|| x86sse1radix2::new_x86_sse_radix2_kernel(cparams))
        .or_else(|| x86sse1radix4::new_x86_sse_radix4_kernel(cparams))
}

pub use self::bitreversal::new_x86_bit_reversal_kernel;
