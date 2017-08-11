//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

//! Defines Radix-2 single precision FFT kernels optimized by using AVX instruction set.
//!
//! AVX expands the register width to 256bit and adds the 256-bit counterparts of most existing instructions.
//!
//! Performances
//! ------------
//!
//! For small transforms ties with a commercial-level FFT library, but tends to be much slower for large transforms.

use super::{Kernel, KernelCreationParams, KernelParams, SliceAccessor, Num};
use super::utils::{StaticParams, StaticParamsConsumer, branch_on_static_params, if_compatible,
                   AlignReqKernelWrapper, AlignReqKernel, AlignInfo};
use super::super::super::simdutils::avx_f32x8_bitxor;

use num_iter::range_step;

use simd::x86::avx::{f32x8, u32x8};

use std::{mem, f32};

pub fn new_x86_avx_f32_radix2_kernel<T>(cparams: &KernelCreationParams) -> Option<Box<Kernel<T>>>
where
    T: Num,
{
    if cparams.radix != 2 {
        return None;
    }

    if_compatible(|| branch_on_static_params(cparams, Factory {}))
}

struct Factory {}
impl StaticParamsConsumer<Option<Box<Kernel<f32>>>> for Factory {
    fn consume<T>(self, cparams: &KernelCreationParams, _: T) -> Option<Box<Kernel<f32>>>
    where
        T: StaticParams,
    {

        match cparams.unit {
            1 if cparams.size % 4 == 0 => Some(Box::new(AlignReqKernelWrapper::new(
                AvxRadix2Kernel1 { cparams: *cparams },
            ))),
            _ => None,
        }
    }
}

/// This Radix-2 kernel is specialized for the case where `unit == 1` and computes two small FFTs in a single iteration.
#[derive(Debug)]
struct AvxRadix2Kernel1 {
    cparams: KernelCreationParams,
}

impl AlignReqKernel<f32> for AvxRadix2Kernel1 {
    fn transform<I: AlignInfo>(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0..cparams.size * 2]) };

        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit, 1);
        assert_eq!(cparams.size % 4, 0);

        let neg_mask: f32x8 = unsafe {
            mem::transmute(u32x8::new(
                0,
                0,
                0x80000000,
                0x80000000,
                0,
                0,
                0x80000000,
                0x80000000,
            ))
        };

        for x in range_step(0, cparams.size * 2, 8) {
            let cur = &mut data[x] as *mut f32 as *mut f32x8;
            // t1a, t1b : Complex<f32> = X[x/2 .. x/2 + 2]
            let t1 = unsafe { I::read(cur) };
            // t2a, t2b = t1b, t1a
            let t2 = f32x8_shuffle!(t1, t1, [2, 3, 8, 9, 6, 7, 12, 13]);
            // t3a, t3b = t1a, -t1b
            let t3 = avx_f32x8_bitxor(t1, neg_mask);
            // t4a, t4b = t2a + t3a, t3b + t3b = t1a + t1b, t1a - t1b
            let t4 = t2 + t3;
            // Y[x/2 .. x/2 + 2] = t4a, t4b
            unsafe { I::write(cur, t4) };
        }
    }
    fn alignment_requirement(&self) -> usize {
        16
    }
}
