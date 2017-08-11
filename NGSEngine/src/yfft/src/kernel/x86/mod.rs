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
#[cfg(target_feature = "avx")]
mod x86avxf32radix2;
#[cfg(target_feature = "avx")]
mod x86avxf32radix4;
mod x86sse1radix2;
mod x86sse1radix4;
mod x86sse1realfft;
mod x86sse2;
#[cfg(target_feature = "sse3")]
mod x86sse3f32radix4;
#[cfg(target_feature = "avx")]
mod x86avxbitreversal;

#[cfg(not(target_feature = "avx"))]
mod x86avxf32radix2 {
    pub fn new_x86_avx_f32_radix2_kernel<T>(_: &super::KernelCreationParams)
                                            -> Option<Box<super::Kernel<T>>> {
        None
    }
}

#[cfg(not(target_feature = "avx"))]
mod x86avxf32radix4 {
    pub fn new_x86_avx_f32_radix4_kernel<T>(_: &super::KernelCreationParams)
                                            -> Option<Box<super::Kernel<T>>> {
        None
    }
}

#[cfg(not(target_feature = "sse3"))]
mod x86sse3f32radix4 {
    pub fn new_x86_sse3_f32_radix4_kernel<T>(_: &super::KernelCreationParams)
                                             -> Option<Box<super::Kernel<T>>> {
        None
    }
}

#[cfg(not(target_feature = "avx"))]
mod x86avxbitreversal {
    pub fn new_x86_avx_bit_reversal_kernel<T>(_: &Vec<usize>) -> Option<Box<super::Kernel<T>>> {
        None
    }
}

pub fn new_x86_kernel<T>(cparams: &KernelCreationParams) -> Option<Box<Kernel<T>>>
    where T: Num
{
    None.or_else(|| x86avxf32radix2::new_x86_avx_f32_radix2_kernel(cparams))
        .or_else(|| x86avxf32radix4::new_x86_avx_f32_radix4_kernel(cparams))
        .or_else(|| x86sse3f32radix4::new_x86_sse3_f32_radix4_kernel(cparams))
        .or_else(|| x86sse2::new_x86_sse2_kernel(cparams))
        .or_else(|| x86sse1radix2::new_x86_sse_radix2_kernel(cparams))
        .or_else(|| x86sse1radix4::new_x86_sse_radix4_kernel(cparams))
}


pub fn new_x86_bit_reversal_kernel<T>(indices: &Vec<usize>) -> Option<Box<Kernel<T>>>
    where T: Num
{
    None.or_else(|| x86avxbitreversal::new_x86_avx_bit_reversal_kernel(indices))
        .or_else(|| bitreversal::new_x86_bit_reversal_kernel(indices))
}


pub fn new_x86_real_fft_pre_post_process_kernel<T>(len: usize, inverse: bool) -> Option<Box<Kernel<T>>>
    where T: Num
{
    None.or_else(|| x86sse1realfft::new_x86_sse_real_fft_pre_post_process_kernel(len, inverse))
}
