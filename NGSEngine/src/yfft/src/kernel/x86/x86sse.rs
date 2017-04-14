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

use immintrin::{__m128, __m128i, xmmintrin, emmintrin};

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

    let full_circle = if cparams.inverse { 2f32 } else { -2f32 };
    let twiddle_delta = Complex::new(Zero::zero(), full_circle * f32::consts::PI / (cparams.radix * cparams.unit) as f32).exp();
    let twiddle_delta2 = twiddle_delta * twiddle_delta;

    let kern: Box<Kernel<f32>> = match (cparams.kernel_type, cparams.radix, cparams.unit) {
        (KernelType::Dit, 2, 1) => Box::new(SseRadix2DitKernel1 {
            cparams: *cparams
        }),
        (KernelType::Dit, 2, unit) if unit % 2 == 0 => Box::new(SseRadix2DitKernel2 {
            cparams: *cparams,
            twiddle_delta: twiddle_delta,
        }),
        _ => return None
    };

    // This is perfectly safe because we can reach here only when T == f32
    // TODO: move this dirty unsafety somewhere outside
    Some(unsafe{mem::transmute(kern)})
}

#[derive(Debug)]
struct SseRadix2DitKernel1 {
    cparams: KernelCreationParams,
}

#[derive(Debug)]
struct SseRadix2DitKernel2 {
    cparams: KernelCreationParams,
    twiddle_delta: Complex<f32>,
}

#[derive(Debug)]
struct SseRadix2DifKernel1 {
    cparams: KernelCreationParams,
}

#[derive(Debug)]
struct SseRadix2DifKernel2 {
    cparams: KernelCreationParams,
    twiddle_delta: Complex<f32>,
}

/// Alternative for `_mm_xor_ps` that doesn't emit unnecessary `call` instructions
#[inline]
fn m128_xor(x: __m128, y: __m128) -> __m128 {
    let x2: __m128i = unsafe { mem::transmute(x) };
    let y2: __m128i = unsafe { mem::transmute(y) };
    let z = emmintrin::_mm_xor_si128(x2, y2);
    unsafe { mem::transmute(z) }
}

#[inline]
fn complex_mul_rrii(x: __m128, y: __m128, neg_mask: __m128) -> __m128 {
    let y_iirr = xmmintrin::_mm_shuffle_ps(
        y, y, 0x4e);

    // (y1a.r * ta.r, y1b.r * tb.r, y1a.i * ta.i, y1b.i * tb.i)
    let t2 = xmmintrin::_mm_mul_ps(x, y);

    // (y1a.r * ta.i, y1b.r * tb.i, y1a.i * ta.r, y1b.i * tb.r)
    let t3 = xmmintrin::_mm_mul_ps(x, y_iirr);

    // (y1a.r * ta.r, y1b.r * tb.r, y1a.r * ta.i, y1b.r * tb.i)
    let t4 = xmmintrin::_mm_shuffle_ps(t2, t3, 0x44);

    // (y1a.i * ta.i, y1b.i * tb.i, y1a.i * ta.r, y1b.i * tb.r)
    let t5 = xmmintrin::_mm_shuffle_ps(t2, t3, 0xee);

    // (-y1a.i * ta.i, -y1b.i * tb.i, y1a.i * ta.r, y1b.i * tb.r)
    let t6 = m128_xor(t5, neg_mask);

    // (y3a.r, y3b.r, y3a.i, y3b.i) =
    // (y1a.r * ta.r - y1a.i * ta.i, y1b.r * tb.r - y1b.i * tb.i,
    //  y1a.r * ta.i + y1a.i * ta.r, y1b.r * tb.i + y1b.i * tb.r)
    xmmintrin::_mm_add_ps(t4, t6)
}

impl Kernel<f32> for SseRadix2DitKernel1 {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        assert_eq!(cparams.kernel_type, KernelType::Dit);
        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit, 1);

        // TODO: check alignment?

        let neg_mask: __m128 = unsafe { mem::transmute(emmintrin::_mm_setr_epi32(0, 0, 0x80000000, 0x80000000)) };

        for x in range_step(0, cparams.size * 2, 4) {
            let cur = &mut data[x] as *mut f32;
            // t1a, t1b : Complex<f32> = X[x/2 .. x/2 + 2]
            let t1 = unsafe { xmmintrin::_mm_load_ps(cur) };
            // t2a, t2b = t1b, t1a
            let t2 = xmmintrin::_mm_shuffle_ps(t1, t1, 0x4e);
            // t3a, t3b = t1a, -t1b
            let t3 = m128_xor(t1, neg_mask);
            // t4a, t4b = t2a + t3a, t3b + t3b = t1a + t1b, t1a - t1b
            let t4 = xmmintrin::_mm_add_ps(t2, t3);
            // Y[x/2 .. x/2 + 2] = t4a, t4b
            unsafe { xmmintrin::_mm_store_ps(cur, t4) };
        }
    }
}

impl Kernel<f32> for SseRadix2DitKernel2 {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        assert_eq!(cparams.kernel_type, KernelType::Dit);
        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit % 2, 0);

        // TODO: check alignment?

        let twiddle_delta = self.twiddle_delta;
        let twiddle_delta2 = twiddle_delta * twiddle_delta;
        let twiddle_delta2_simd = xmmintrin::_mm_setr_ps(
            twiddle_delta2.re, twiddle_delta2.re,
            twiddle_delta2.im, twiddle_delta2.im);
        let twiddle_1_init = xmmintrin::_mm_setr_ps(
            0f32, twiddle_delta.re,
            0f32, twiddle_delta.im);

        let neg_mask: __m128 = unsafe { mem::transmute(emmintrin::_mm_setr_epi32(0x80000000, 0x80000000, 0, 0)) };

        for x in range_step(0, cparams.size, cparams.unit * 2) {
            let mut twiddle_1 = twiddle_1_init;
            for y in range_step(0, cparams.unit * 2, 4) {
                let cur1 = &mut data[x + y] as *mut f32;
                let cur2 = &mut data[x + y + cparams.unit * 2] as *mut f32;

                // x1a, x1b : Complex<f32> = X[x1/2 .. x1/2 + 2]
                // (x1a.r, x1a.i, x1b.r, x1b.i)
                let x1 = unsafe { xmmintrin::_mm_load_ps(cur1) };
                // y1a, y1b : Complex<f32> = X[x2/2 .. x2/2 + 2]
                // (y1a.r, y1a.i, y2a.r, y2a.i)
                let y1 = unsafe { xmmintrin::_mm_load_ps(cur2) };

                // apply twiddle factor
                // (y1a.r, y1b.r, y1a.i, y1b.i)
                let y2t1 = xmmintrin::_mm_shuffle_ps(y1, y1, 0xd8);

                let y3 = complex_mul_rrii(y2t1, twiddle_1, neg_mask);

                // perform size-2 FFT
                // (y3a.r, y3a.i, y3b.r, y3b.i)
                let y3t1 = xmmintrin::_mm_shuffle_ps(y1, y1, 0xd8);

                let x3 = xmmintrin::_mm_add_ps(x1, y3t1);
                let y3 = xmmintrin::_mm_sub_ps(x1, y3t1);

                unsafe { xmmintrin::_mm_store_ps(cur1, x3) };
                unsafe { xmmintrin::_mm_store_ps(cur1, y3) };
            }
            twiddle_1 = complex_mul_rrii(twiddle_1, twiddle_delta2_simd, neg_mask);
        }
    }
}

impl Kernel<f32> for SseRadix2DifKernel1 {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        assert_eq!(cparams.kernel_type, KernelType::Dif);
        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit, 1);

        unimplemented!()
    }
}

impl Kernel<f32> for SseRadix2DifKernel2 {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        assert_eq!(cparams.kernel_type, KernelType::Dif);
        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit % 2, 0);

        unimplemented!()
    }
}
