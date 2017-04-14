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
use super::{f32x4_bitxor};

use num_complex::Complex;
use num_traits::{Zero, One, FloatConst};
use num_iter::range_step;

use simd::{f32x4, i32x4};

use super::super::super::mul_pos_i;

use std::fmt::Debug;
use std::marker::PhantomData;
use std::any::TypeId;
use std::mem;
use std::f32;
use std::ops;

pub fn new_x86_sse_kernel<T>(cparams: &KernelCreationParams) -> Option<Box<Kernel<T>>>
    where T : Num {

    // Rust doesn't have partial specialization of generics yet...
    if TypeId::of::<T>() != TypeId::of::<f32>() {
        return None
    }

    let kern: Box<Kernel<f32>> = match (cparams.kernel_type, cparams.radix, cparams.unit) {
        (KernelType::Dit, 2, 1) => Box::new(SseRadix2DitKernel1 {
            cparams: *cparams
        }),
        (KernelType::Dit, 2, unit) if unit % 2 == 0 => Box::new(SseRadix2DitKernel2::new(cparams)),
        _ => return None
    };

    // This is perfectly safe because we can reach here only when T == f32
    // TODO: move this dirty unsafety somewhere outside
    Some(unsafe{mem::transmute(kern)})
}

#[inline]
fn complex_mul_rrii(x: f32x4, y: f32x4, neg_mask: f32x4) -> f32x4 {
    let y_iirr = f32x4_shuffle!(y, y, [2, 3, 4, 5]);

    // (y1a.r * ta.r, y1b.r * tb.r, y1a.i * ta.i, y1b.i * tb.i)
    let t2 = x * y;

    // (y1a.r * ta.i, y1b.r * tb.i, y1a.i * ta.r, y1b.i * tb.r)
    let t3 = x * y_iirr;

    // (y1a.r * ta.r, y1b.r * tb.r, y1a.r * ta.i, y1b.r * tb.i)
    let t4 = f32x4_shuffle!(t2, t3, [0, 1, 4, 5]);

    // (y1a.i * ta.i, y1b.i * tb.i, y1a.i * ta.r, y1b.i * tb.r)
    let t5 = f32x4_shuffle!(t2, t3, [2, 3, 6, 7]);

    // (-y1a.i * ta.i, -y1b.i * tb.i, y1a.i * ta.r, y1b.i * tb.r)
    let t6 = f32x4_bitxor(t5, neg_mask);

    // (y3a.r, y3b.r, y3a.i, y3b.i) =
    // (y1a.r * ta.r - y1a.i * ta.i, y1b.r * tb.r - y1b.i * tb.i,
    //  y1a.r * ta.i + y1a.i * ta.r, y1b.r * tb.i + y1b.i * tb.r)
    t4 + t6
}

#[test]
fn test_complex_mul_rrii() {
    let neg_mask_raw: [u32; 4] = [0x80000000, 0x80000000, 0, 0];
    let neg_mask = unsafe { *(&neg_mask_raw as *const u32 as *const f32x4) };

    let c1: Complex<f32> = Complex::new(123f32, 456f32);
    let c2: Complex<f32> = Complex::new(789f32, 135f32);
    let c3: Complex<f32> = Complex::new(114f32, 514f32);
    let c4: Complex<f32> = Complex::new(987f32, 654f32);

    let d1 = c1 * c3;
    let d2 = c2 * c4;

    let x = f32x4::new(c1.re, c2.re, c1.im, c2.im);
    let y = f32x4::new(c3.re, c4.re, c3.im, c4.im);
    let z = complex_mul_rrii(x, y, neg_mask);

    assert_eq!(super::f32x4_to_array(z), [d1.re, d2.re, d1.im, d2.im]);
}

#[derive(Debug)]
struct SseRadix2DitKernel1 {
    cparams: KernelCreationParams,
}

impl Kernel<f32> for SseRadix2DitKernel1 {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        assert_eq!(cparams.kernel_type, KernelType::Dit);
        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit, 1);

        // TODO: check alignment?

        let neg_mask_raw: [u32; 4] = [0, 0, 0x80000000, 0x80000000];
        let neg_mask = unsafe { *(&neg_mask_raw as *const u32 as *const f32x4) };

        for x in range_step(0, cparams.size * 2, 4) {
            let cur = &mut data[x] as *mut f32 as *mut f32x4;
            // t1a, t1b : Complex<f32> = X[x/2 .. x/2 + 2]
            let t1 = unsafe { *cur };
            // t2a, t2b = t1b, t1a
            let t2 = f32x4_shuffle!(t1, t1, [2, 3, 4, 5]);
            // t3a, t3b = t1a, -t1b
            let t3 = f32x4_bitxor(t1, neg_mask);
            // t4a, t4b = t2a + t3a, t3b + t3b = t1a + t1b, t1a - t1b
            let t4 = t2 + t3;
            // Y[x/2 .. x/2 + 2] = t4a, t4b
            unsafe { *cur = t4 };
        }
    }
}

#[derive(Debug)]
struct SseRadix2DitKernel2 {
    cparams: KernelCreationParams,
    twiddles: Vec<f32x4>
}

impl SseRadix2DitKernel2 {
    fn new(cparams: &KernelCreationParams) -> Self {
        let full_circle = if cparams.inverse { 2f32 } else { -2f32 };
        let twiddle_delta = Complex::new(Zero::zero(), full_circle *
            f32::consts::PI / (cparams.radix * cparams.unit) as f32).exp();
        let twiddles = range_step(0, cparams.unit, 2)
            .map(|i| {
                let c1 = Complex::new(Zero::zero(), full_circle * (i) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                let c2 = Complex::new(Zero::zero(), full_circle * (i + 1) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                // rrii format
                f32x4::new(c1.re, c2.re, c1.im, c2.im)
            })
            .collect();

        Self {
            cparams: *cparams,
            twiddles: twiddles,
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

        let twiddles = unsafe { SliceAccessor::new(self.twiddles.as_slice()) };

        let neg_mask_raw: [u32; 4] = [0x80000000, 0x80000000, 0, 0];
        let neg_mask = unsafe { *(&neg_mask_raw as *const u32 as *const f32x4) };

        for x in range_step(0, cparams.size * 2, cparams.unit * 4) {
            for y in range_step(0, cparams.unit * 2, 4) {
                let cur1 = &mut data[x + y] as *mut f32 as *mut f32x4;
                let cur2 = &mut data[x + y + cparams.unit * 2] as *mut f32 as *mut f32x4;
                let twiddle_1 = twiddles[y >> 2];

                // x1a, x1b : Complex<f32> = X[x1/2 .. x1/2 + 2]
                // (x1a.r, x1a.i, x1b.r, x1b.i)
                let x1 = unsafe { *cur1 };
                // y1a, y1b : Complex<f32> = X[x2/2 .. x2/2 + 2]
                // (y1a.r, y1a.i, y1b.r, y1b.i)
                let y1 = unsafe { *cur2 };

                // apply twiddle factor
                // (y1a.r, y1b.r, y1a.i, y1b.i)
                let y2t1 = f32x4_shuffle!(y1, y1, [0, 2, 5, 7]);

                let y3 = complex_mul_rrii(y2t1, twiddle_1, neg_mask);

                // perform size-2 FFT
                // (y3a.r, y3a.i, y3b.r, y3b.i)
                let y3t1 = f32x4_shuffle!(y3, y3, [0, 2, 5, 7]);

                let x3 = x1 + y3t1;
                let y3 = x1 - y3t1;

                unsafe { *cur1 = x3 };
                unsafe { *cur2 = y3 };
            }
        }
    }
}

#[derive(Debug)]
struct SseRadix2DifKernel1 {
    cparams: KernelCreationParams,
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

#[derive(Debug)]
struct SseRadix2DifKernel2 {
    cparams: KernelCreationParams,
    twiddle_delta: Complex<f32>,
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
