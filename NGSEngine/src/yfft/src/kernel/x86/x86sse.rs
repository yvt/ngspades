//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

//! Defines FFT kernels optimized by using SSE instruction set.
//!
//! Performances
//! ------------
//!
//! Yet to be measured.

use super::{Kernel, KernelCreationParams, KernelParams, KernelType, SliceAccessor, Num};

use num_complex::Complex;
use num_traits::{Zero, One, FloatConst};
use num_iter::range_step;

use immintrin::{__m128};

use super::super::super::mul_pos_i;

use std::fmt::Debug;
use std::marker::PhantomData;
use std::any::TypeId;
use std::mem;
use std::f32;

pub fn new_x86_sse_kernel<T>(cparams: &KernelCreationParams) -> Option<Box<Kernel<T>>>
    where T : Num {

    // Rust doesn't have partial specialization of generics yet...
    if TypeId::of::<T>() != TypeId::of::<f32>() {
        return None
    }

    // Not implemented, anyway
    return None;

    let full_circle = if cparams.inverse { 2f32 } else { -2f32 };

    let kern: Box<Kernel<f32>> = match cparams.kernel_type {
        KernelType::Dit => Box::new(SseDitKernel {
            cparams: *cparams,
            twiddle_delta: Complex::new(Zero::zero(),
                full_circle * f32::consts::PI / (cparams.radix * cparams.unit) as f32).exp(),
        }),
        KernelType::Dif => Box::new(SseDifKernel {
            cparams: *cparams,
            twiddle_delta: Complex::new(Zero::zero(),
                full_circle * f32::consts::PI / (cparams.radix * cparams.unit) as f32).exp(),
        }),
    } ;

    // This is perfectly safe because we can reach here only when T == f32
    Some(unsafe{mem::transmute(kern)})
}

#[derive(Debug)]
struct SseDitKernel {
    cparams: KernelCreationParams,
    twiddle_delta: Complex<f32>,
}

#[derive(Debug)]
struct SseDifKernel {
    cparams: KernelCreationParams,
    twiddle_delta: Complex<f32>,
}

impl Kernel<f32> for SseDitKernel {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]);

        let twiddle_delta = self.twiddle_delta;
        unimplemented!()
    }
}

impl Kernel<f32> for SseDifKernel {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]);

        let twiddle_delta = self.twiddle_delta;
        unimplemented!()
    }
}
