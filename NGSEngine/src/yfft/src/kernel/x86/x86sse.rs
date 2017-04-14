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
use super::super::super::simdutils::{f32x4_bitxor, f32x4_complex_mul_rrii};

use num_complex::Complex;
use num_iter::range_step;

use simd::f32x4;

use std::any::TypeId;
use std::{mem, f32, fmt};

pub fn new_x86_sse_kernel<T>(cparams: &KernelCreationParams) -> Option<Box<Kernel<T>>>
    where T : Num {

    // Rust doesn't have partial specialization of generics yet...
    if TypeId::of::<T>() != TypeId::of::<f32>() {
        return None
    }

    let kern: Box<Kernel<f32>> = match (cparams.kernel_type, cparams.radix, cparams.unit, cparams.inverse) {
        (KernelType::Dit, 4, unit, false) if unit % 4 == 0 =>
            Box::new(SseRadix4DitKernel4::new(cparams, StaticParamsForward{})),
        (KernelType::Dit, 4, unit, true)  if unit % 4 == 0 =>
            Box::new(SseRadix4DitKernel4::new(cparams, StaticParamsBackward{})),
        (_, 2, 1, _) => Box::new(SseRadix2Kernel1 { cparams: *cparams }),
        (KernelType::Dif, 4, unit, false) if unit % 4 == 0 =>
            Box::new(SseRadix4DifKernel4::new(cparams, StaticParamsForward{})),
        (KernelType::Dif, 4, unit, true)  if unit % 4 == 0 =>
            Box::new(SseRadix4DifKernel4::new(cparams, StaticParamsBackward{})),
        (_, 2, 1, _) => Box::new(SseRadix2Kernel1 { cparams: *cparams }),
        (KernelType::Dit, 2, unit, _) if unit % 4 == 0 => Box::new(SseRadix2DitKernel3::<>::new(cparams)),
        (KernelType::Dif, 2, unit, _) if unit % 4 == 0 => Box::new(SseRadix2DifKernel3::<>::new(cparams)),
        (KernelType::Dit, 2, unit, _) if unit % 2 == 0 => Box::new(SseRadix2DitKernel2::<>::new(cparams)),
        (KernelType::Dif, 2, unit, _) if unit % 2 == 0 => Box::new(SseRadix2DifKernel2::<>::new(cparams)),
        _ => return None
    };

    // This is perfectly safe because we can reach here only when T == f32
    // TODO: move this dirty unsafety somewhere outside
    Some(unsafe{mem::transmute(kern)})
}

trait StaticParams : fmt::Debug {
    fn inverse(&self) -> bool;
}

#[derive(Debug)] struct StaticParamsForward {}
impl StaticParams for StaticParamsForward {
    #[inline] fn inverse(&self) -> bool { false }
}

#[derive(Debug)] struct StaticParamsBackward {}
impl StaticParams for StaticParamsBackward {
    #[inline] fn inverse(&self) -> bool { true }
}

#[derive(Debug)]
struct SseRadix2Kernel1 {
    cparams: KernelCreationParams,
}

impl Kernel<f32> for SseRadix2Kernel1 {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

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
        let twiddles = range_step(0, cparams.unit, 2)
            .map(|i| {
                let c1 = Complex::new(0f32, full_circle * (i) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                let c2 = Complex::new(0f32, full_circle * (i + 1) as f32 /
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
            for y in 0 .. cparams.unit / 2 {
                let cur1 = &mut data[x + y * 4] as *mut f32 as *mut f32x4;
                let cur2 = &mut data[x + y * 4 + cparams.unit * 2] as *mut f32 as *mut f32x4;
                let twiddle_1 = twiddles[y];

                // x1a, x1b : Complex<f32> = X[x1/2 .. x1/2 + 2]
                // (x1a.r, x1a.i, x1b.r, x1b.i)
                let x1 = unsafe { *cur1 };
                // y1a, y1b : Complex<f32> = X[x2/2 .. x2/2 + 2]
                // (y1a.r, y1a.i, y1b.r, y1b.i)
                let y1 = unsafe { *cur2 };

                // apply twiddle factor
                // (y1a.r, y1b.r, y1a.i, y1b.i)
                let y2t1 = f32x4_shuffle!(y1, y1, [0, 2, 5, 7]);

                let y3 = f32x4_complex_mul_rrii(y2t1, twiddle_1, neg_mask);

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
struct SseRadix2DitKernel3 {
    cparams: KernelCreationParams,
    twiddles: Vec<f32x4>
}

impl SseRadix2DitKernel3 {
    fn new(cparams: &KernelCreationParams) -> Self {
        let full_circle = if cparams.inverse { 2f32 } else { -2f32 };
        let twiddles = range_step(0, cparams.unit, 2)
            .map(|i| {
                let k = i / 4 * 4;
                let c1 = Complex::new(0f32, full_circle * (k) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                let c2 = Complex::new(0f32, full_circle * (k + 1) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                let c3 = Complex::new(0f32, full_circle * (k + 2) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                let c4 = Complex::new(0f32, full_circle * (k + 3) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                // rrrr-iiii format
                // TODO: more efficient creation
                if i % 4 != 0 {
                    f32x4::new(c1.im, c2.im, c3.im, c4.im)
                } else {
                    f32x4::new(c1.re, c2.re, c3.re, c4.re)
                }
            })
            .collect();

        Self {
            cparams: *cparams,
            twiddles: twiddles,
        }
    }
}

impl Kernel<f32> for SseRadix2DitKernel3 {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        assert_eq!(cparams.kernel_type, KernelType::Dit);
        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit % 4, 0);

        // TODO: check alignment?

        let twiddles = unsafe { SliceAccessor::new(self.twiddles.as_slice()) };

        for x in range_step(0, cparams.size * 2, cparams.unit * 4) {
            for y in 0 .. cparams.unit / 4 {
                let cur1a = &mut data[x + y * 8] as *mut f32 as *mut f32x4;
                let cur1b = &mut data[x + y * 8 + 4] as *mut f32 as *mut f32x4;
                let cur2a = &mut data[x + y * 8 + cparams.unit * 2] as *mut f32 as *mut f32x4;
                let cur2b = &mut data[x + y * 8 + cparams.unit * 2 + 4] as *mut f32 as *mut f32x4;
                let twiddle_r = twiddles[y * 2];
                let twiddle_i = twiddles[y * 2 + 1];

                let x1a = unsafe { *cur1a };
                let x1b = unsafe { *cur1b };
                let y1a = unsafe { *cur2a };
                let y1b = unsafe { *cur2b };

                // convert riri-riri to rrrr-iiii (shufps)
                let x2r = f32x4_shuffle!(x1a, x1b, [0, 2, 4, 6]);
                let x2i = f32x4_shuffle!(x1a, x1b, [1, 3, 5, 7]);
                let y2r = f32x4_shuffle!(y1a, y1b, [0, 2, 4, 6]);
                let y2i = f32x4_shuffle!(y1a, y1b, [1, 3, 5, 7]);

                // apply twiddle factor
                let x3r = x2r;
                let x3i = x2i;
                let y3r = y2r * twiddle_r - y2i * twiddle_i;
                let y3i = y2r * twiddle_i + y2i * twiddle_r;

                // perform size-2 FFT
                let x4r = x3r + y3r;
                let x4i = x3i + y3i;
                let y4r = x3r - y3r;
                let y4i = x3i - y3i;

                // convert to rrrr-iiii to riri-riri (unpcklps/unpckups)
                let x5a = f32x4_shuffle!(x4r, x4i, [0, 4, 1, 5]);
                let x5b = f32x4_shuffle!(x4r, x4i, [2, 6, 3, 7]);
                let y5a = f32x4_shuffle!(y4r, y4i, [0, 4, 1, 5]);
                let y5b = f32x4_shuffle!(y4r, y4i, [2, 6, 3, 7]);

                unsafe { *cur1a = x5a };
                unsafe { *cur1b = x5b };
                unsafe { *cur2a = y5a };
                unsafe { *cur2b = y5b };
            }
        }
    }
}

#[derive(Debug)]
struct SseRadix2DifKernel2 {
    cparams: KernelCreationParams,
    twiddles: Vec<f32x4>
}

impl SseRadix2DifKernel2 {
    fn new(cparams: &KernelCreationParams) -> Self {
        let full_circle = if cparams.inverse { 2f32 } else { -2f32 };
        let twiddles = range_step(0, cparams.unit, 2)
            .map(|i| {
                let c1 = Complex::new(0f32, full_circle * (i) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                let c2 = Complex::new(0f32, full_circle * (i + 1) as f32 /
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

impl Kernel<f32> for SseRadix2DifKernel2 {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        assert_eq!(cparams.kernel_type, KernelType::Dif);
        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit % 2, 0);

        // TODO: check alignment?

        let twiddles = unsafe { SliceAccessor::new(self.twiddles.as_slice()) };

        let neg_mask_raw: [u32; 4] = [0x80000000, 0x80000000, 0, 0];
        let neg_mask = unsafe { *(&neg_mask_raw as *const u32 as *const f32x4) };

        for x in range_step(0, cparams.size * 2, cparams.unit * 4) {
            for y in 0 .. cparams.unit / 2 {
                let cur1 = &mut data[x + y * 4] as *mut f32 as *mut f32x4;
                let cur2 = &mut data[x + y * 4 + cparams.unit * 2] as *mut f32 as *mut f32x4;
                let twiddle_1 = twiddles[y];

                // x1a, x1b : Complex<f32> = X[x1/2 .. x1/2 + 2]
                // (x1a.r, x1a.i, x1b.r, x1b.i)
                let x1 = unsafe { *cur1 };
                // y1a, y1b : Complex<f32> = X[x2/2 .. x2/2 + 2]
                // (y1a.r, y1a.i, y1b.r, y1b.i)
                let y1 = unsafe { *cur2 };

                // perform size-2 FFT
                let x2 = x1 + y1;
                let y2 = x1 - y1;

                // apply twiddle factor
                // (y1a.r, y1b.r, y1a.i, y1b.i)
                let y2t1 = f32x4_shuffle!(y2, y2, [0, 2, 5, 7]);

                let y3t1 = f32x4_complex_mul_rrii(y2t1, twiddle_1, neg_mask);

                let x3 = x2;
                let y3 = f32x4_shuffle!(y3t1, y3t1, [0, 2, 5, 7]);

                unsafe { *cur1 = x3 };
                unsafe { *cur2 = y3 };
            }
        }
    }
}

#[derive(Debug)]
struct SseRadix2DifKernel3 {
    cparams: KernelCreationParams,
    twiddles: Vec<f32x4>
}

impl SseRadix2DifKernel3 {
    fn new(cparams: &KernelCreationParams) -> Self {
        let full_circle = if cparams.inverse { 2f32 } else { -2f32 };
        let twiddles = range_step(0, cparams.unit, 2)
            .map(|i| {
                let k = i / 4 * 4;
                let c1 = Complex::new(0f32, full_circle * (k) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                let c2 = Complex::new(0f32, full_circle * (k + 1) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                let c3 = Complex::new(0f32, full_circle * (k + 2) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                let c4 = Complex::new(0f32, full_circle * (k + 3) as f32 /
                    (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
                // rrrr-iiii format
                // TODO: more efficient creation
                if i % 4 != 0 {
                    f32x4::new(c1.im, c2.im, c3.im, c4.im)
                } else {
                    f32x4::new(c1.re, c2.re, c3.re, c4.re)
                }
            })
            .collect();

        Self {
            cparams: *cparams,
            twiddles: twiddles,
        }
    }
}

impl Kernel<f32> for SseRadix2DifKernel3 {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        assert_eq!(cparams.kernel_type, KernelType::Dif);
        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit % 4, 0);

        // TODO: check alignment?

        let twiddles = unsafe { SliceAccessor::new(self.twiddles.as_slice()) };

        for x in range_step(0, cparams.size * 2, cparams.unit * 4) {
            for y in 0 .. cparams.unit / 4 {
                let cur1a = &mut data[x + y * 8] as *mut f32 as *mut f32x4;
                let cur1b = &mut data[x + y * 8 + 4] as *mut f32 as *mut f32x4;
                let cur2a = &mut data[x + y * 8 + cparams.unit * 2] as *mut f32 as *mut f32x4;
                let cur2b = &mut data[x + y * 8 + cparams.unit * 2 + 4] as *mut f32 as *mut f32x4;
                let twiddle_r = twiddles[y * 2];
                let twiddle_i = twiddles[y * 2 + 1];

                let x1a = unsafe { *cur1a };
                let x1b = unsafe { *cur1b };
                let y1a = unsafe { *cur2a };
                let y1b = unsafe { *cur2b };

                // convert riri-riri to rrrr-iiii (shufps)
                let x2r = f32x4_shuffle!(x1a, x1b, [0, 2, 4, 6]);
                let x2i = f32x4_shuffle!(x1a, x1b, [1, 3, 5, 7]);
                let y2r = f32x4_shuffle!(y1a, y1b, [0, 2, 4, 6]);
                let y2i = f32x4_shuffle!(y1a, y1b, [1, 3, 5, 7]);

                // perform size-2 FFT
                let x3r = x2r + y2r;
                let x3i = x2i + y2i;
                let y3r = x2r - y2r;
                let y3i = x2i - y2i;

                // apply twiddle factor
                let x4r = x3r;
                let x4i = x3i;
                let y4r = y3r * twiddle_r - y3i * twiddle_i;
                let y4i = y3r * twiddle_i + y3i * twiddle_r;

                // convert to rrrr-iiii to riri-riri (unpcklps/unpckups)
                let x5a = f32x4_shuffle!(x4r, x4i, [0, 4, 1, 5]);
                let x5b = f32x4_shuffle!(x4r, x4i, [2, 6, 3, 7]);
                let y5a = f32x4_shuffle!(y4r, y4i, [0, 4, 1, 5]);
                let y5b = f32x4_shuffle!(y4r, y4i, [2, 6, 3, 7]);

                unsafe { *cur1a = x5a };
                unsafe { *cur1b = x5b };
                unsafe { *cur2a = y5a };
                unsafe { *cur2b = y5b };
            }
        }
    }
}

#[derive(Debug)]
struct SseRadix4DitKernel4<T: StaticParams> {
    cparams: KernelCreationParams,
    twiddles: Vec<f32x4>,
    sparams: T
}

impl<T: StaticParams> SseRadix4DitKernel4<T> {
    fn new(cparams: &KernelCreationParams, sparams: T) -> Self {
        let full_circle = if cparams.inverse { 2f32 } else { -2f32 };
        let mut twiddles = Vec::new();
        for i in range_step(0, cparams.unit, 4) {
            let c1 = Complex::new(0f32, full_circle * (i) as f32 /
                (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
            let c2 = Complex::new(0f32, full_circle * (i + 1) as f32 /
                (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
            let c3 = Complex::new(0f32, full_circle * (i + 2) as f32 /
                (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
            let c4 = Complex::new(0f32, full_circle * (i + 3) as f32 /
                (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
            // rrrr-iiii format
            twiddles.push(f32x4::new(c1.re, c2.re, c3.re, c4.re));
            twiddles.push(f32x4::new(c1.im, c2.im, c3.im, c4.im));

            let c12 = c1 * c1;
            let c22 = c2 * c2;
            let c32 = c3 * c3;
            let c42 = c4 * c4;
            twiddles.push(f32x4::new(c12.re, c22.re, c32.re, c42.re));
            twiddles.push(f32x4::new(c12.im, c22.im, c32.im, c42.im));

            let c13 = c12 * c1;
            let c23 = c22 * c2;
            let c33 = c32 * c3;
            let c43 = c42 * c4;
            twiddles.push(f32x4::new(c13.re, c23.re, c33.re, c43.re));
            twiddles.push(f32x4::new(c13.im, c23.im, c33.im, c43.im));
        }

        Self {
            cparams: *cparams,
            twiddles: twiddles,
            sparams: sparams,
        }
    }
}

impl<T: StaticParams> Kernel<f32> for SseRadix4DitKernel4<T> {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        assert_eq!(cparams.kernel_type, KernelType::Dit);
        assert_eq!(cparams.radix, 4);
        assert_eq!(cparams.unit % 4, 0);

        // TODO: check alignment?

        let twiddles = unsafe { SliceAccessor::new(self.twiddles.as_slice()) };

        for x in range_step(0, cparams.size * 2, cparams.unit * 8) {
            for y in 0 .. cparams.unit / 4 {
                let cur1a = &mut data[x + y * 8] as *mut f32 as *mut f32x4;
                let cur1b = &mut data[x + y * 8 + 4] as *mut f32 as *mut f32x4;
                let cur2a = &mut data[x + y * 8 + cparams.unit * 2] as *mut f32 as *mut f32x4;
                let cur2b = &mut data[x + y * 8 + cparams.unit * 2 + 4] as *mut f32 as *mut f32x4;
                let cur3a = &mut data[x + y * 8 + cparams.unit * 4] as *mut f32 as *mut f32x4;
                let cur3b = &mut data[x + y * 8 + cparams.unit * 4 + 4] as *mut f32 as *mut f32x4;
                let cur4a = &mut data[x + y * 8 + cparams.unit * 6] as *mut f32 as *mut f32x4;
                let cur4b = &mut data[x + y * 8 + cparams.unit * 6 + 4] as *mut f32 as *mut f32x4;
                let twiddle1_r = twiddles[y * 6];
                let twiddle1_i = twiddles[y * 6 + 1];
                let twiddle2_r = twiddles[y * 6 + 2];
                let twiddle2_i = twiddles[y * 6 + 3];
                let twiddle3_r = twiddles[y * 6 + 4];
                let twiddle3_i = twiddles[y * 6 + 5];

                let x1a = unsafe { *cur1a }; let x1b = unsafe { *cur1b };
                let y1a = unsafe { *cur2a }; let y1b = unsafe { *cur2b };
                let z1a = unsafe { *cur3a }; let z1b = unsafe { *cur3b };
                let w1a = unsafe { *cur4a }; let w1b = unsafe { *cur4b };

                // convert riri-riri to rrrr-iiii (shufps)
                let x2r = f32x4_shuffle!(x1a, x1b, [0, 2, 4, 6]);
                let x2i = f32x4_shuffle!(x1a, x1b, [1, 3, 5, 7]);
                let y2r = f32x4_shuffle!(y1a, y1b, [0, 2, 4, 6]);
                let y2i = f32x4_shuffle!(y1a, y1b, [1, 3, 5, 7]);
                let z2r = f32x4_shuffle!(z1a, z1b, [0, 2, 4, 6]);
                let z2i = f32x4_shuffle!(z1a, z1b, [1, 3, 5, 7]);
                let w2r = f32x4_shuffle!(w1a, w1b, [0, 2, 4, 6]);
                let w2i = f32x4_shuffle!(w1a, w1b, [1, 3, 5, 7]);

                // apply twiddle factor
                let x3r = x2r;
                let x3i = x2i;
                let y3r = y2r * twiddle1_r - y2i * twiddle1_i;
                let y3i = y2r * twiddle1_i + y2i * twiddle1_r;
                let z3r = z2r * twiddle2_r - z2i * twiddle2_i;
                let z3i = z2r * twiddle2_i + z2i * twiddle2_r;
                let w3r = w2r * twiddle3_r - w2i * twiddle3_i;
                let w3i = w2r * twiddle3_i + w2i * twiddle3_r;

                // perform size-4 FFT
                // TODO; change this to size-4
                let x4r = x3r + z3r; let x4i = x3i + z3i;
                let y4r = y3r + w3r; let y4i = y3i + w3i;
                let z4r = x3r - z3r; let z4i = x3i - z3i;
                let w4r = y3r - w3r; let w4i = y3i - w3i;

                let x5r = x4r + y4r; let x5i = x4i + y4i;
                let z5r = x4r - y4r; let z5i = x4i - y4i;
                let (y5r, y5i, w5r, w5i) = if self.sparams.inverse() {
                    (z4r - w4i, z4i + w4r, z4r + w4i, z4i - w4r)
                } else {
                    (z4r + w4i, z4i - w4r, z4r - w4i, z4i + w4r)
                };

                // convert to rrrr-iiii to riri-riri (unpcklps/unpckups)
                let x6a = f32x4_shuffle!(x5r, x5i, [0, 4, 1, 5]);
                let x6b = f32x4_shuffle!(x5r, x5i, [2, 6, 3, 7]);
                let y6a = f32x4_shuffle!(y5r, y5i, [0, 4, 1, 5]);
                let y6b = f32x4_shuffle!(y5r, y5i, [2, 6, 3, 7]);
                let z6a = f32x4_shuffle!(z5r, z5i, [0, 4, 1, 5]);
                let z6b = f32x4_shuffle!(z5r, z5i, [2, 6, 3, 7]);
                let w6a = f32x4_shuffle!(w5r, w5i, [0, 4, 1, 5]);
                let w6b = f32x4_shuffle!(w5r, w5i, [2, 6, 3, 7]);

                unsafe { *cur1a = x6a }; unsafe { *cur1b = x6b };
                unsafe { *cur2a = y6a }; unsafe { *cur2b = y6b };
                unsafe { *cur3a = z6a }; unsafe { *cur3b = z6b };
                unsafe { *cur4a = w6a }; unsafe { *cur4b = w6b };
            }
        }
    }
}

#[derive(Debug)]
struct SseRadix4DifKernel4<T: StaticParams> {
    cparams: KernelCreationParams,
    twiddles: Vec<f32x4>,
    sparams: T
}

impl<T: StaticParams> SseRadix4DifKernel4<T> {
    fn new(cparams: &KernelCreationParams, sparams: T) -> Self {
        let full_circle = if cparams.inverse { 2f32 } else { -2f32 };
        let mut twiddles = Vec::new();
        for i in range_step(0, cparams.unit, 4) {
            let c1 = Complex::new(0f32, full_circle * (i) as f32 /
                (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
            let c2 = Complex::new(0f32, full_circle * (i + 1) as f32 /
                (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
            let c3 = Complex::new(0f32, full_circle * (i + 2) as f32 /
                (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
            let c4 = Complex::new(0f32, full_circle * (i + 3) as f32 /
                (cparams.radix * cparams.unit) as f32 * f32::consts::PI).exp();
            // rrrr-iiii format
            twiddles.push(f32x4::new(c1.re, c2.re, c3.re, c4.re));
            twiddles.push(f32x4::new(c1.im, c2.im, c3.im, c4.im));

            let c12 = c1 * c1;
            let c22 = c2 * c2;
            let c32 = c3 * c3;
            let c42 = c4 * c4;
            twiddles.push(f32x4::new(c12.re, c22.re, c32.re, c42.re));
            twiddles.push(f32x4::new(c12.im, c22.im, c32.im, c42.im));

            let c13 = c12 * c1;
            let c23 = c22 * c2;
            let c33 = c32 * c3;
            let c43 = c42 * c4;
            twiddles.push(f32x4::new(c13.re, c23.re, c33.re, c43.re));
            twiddles.push(f32x4::new(c13.im, c23.im, c33.im, c43.im));
        }

        Self {
            cparams: *cparams,
            twiddles: twiddles,
            sparams: sparams,
        }
    }
}

impl<T: StaticParams> Kernel<f32> for SseRadix4DifKernel4<T> {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        assert_eq!(cparams.kernel_type, KernelType::Dif);
        assert_eq!(cparams.radix, 4);
        assert_eq!(cparams.unit % 4, 0);

        // TODO: check alignment?

        let twiddles = unsafe { SliceAccessor::new(self.twiddles.as_slice()) };

        for x in range_step(0, cparams.size * 2, cparams.unit * 8) {
            for y in 0 .. cparams.unit / 4 {
                let cur1a = &mut data[x + y * 8] as *mut f32 as *mut f32x4;
                let cur1b = &mut data[x + y * 8 + 4] as *mut f32 as *mut f32x4;
                let cur2a = &mut data[x + y * 8 + cparams.unit * 2] as *mut f32 as *mut f32x4;
                let cur2b = &mut data[x + y * 8 + cparams.unit * 2 + 4] as *mut f32 as *mut f32x4;
                let cur3a = &mut data[x + y * 8 + cparams.unit * 4] as *mut f32 as *mut f32x4;
                let cur3b = &mut data[x + y * 8 + cparams.unit * 4 + 4] as *mut f32 as *mut f32x4;
                let cur4a = &mut data[x + y * 8 + cparams.unit * 6] as *mut f32 as *mut f32x4;
                let cur4b = &mut data[x + y * 8 + cparams.unit * 6 + 4] as *mut f32 as *mut f32x4;
                let twiddle1_r = twiddles[y * 6];
                let twiddle1_i = twiddles[y * 6 + 1];
                let twiddle2_r = twiddles[y * 6 + 2];
                let twiddle2_i = twiddles[y * 6 + 3];
                let twiddle3_r = twiddles[y * 6 + 4];
                let twiddle3_i = twiddles[y * 6 + 5];

                let x1a = unsafe { *cur1a }; let x1b = unsafe { *cur1b };
                let y1a = unsafe { *cur2a }; let y1b = unsafe { *cur2b };
                let z1a = unsafe { *cur3a }; let z1b = unsafe { *cur3b };
                let w1a = unsafe { *cur4a }; let w1b = unsafe { *cur4b };

                // convert riri-riri to rrrr-iiii (shufps)
                let x2r = f32x4_shuffle!(x1a, x1b, [0, 2, 4, 6]);
                let x2i = f32x4_shuffle!(x1a, x1b, [1, 3, 5, 7]);
                let y2r = f32x4_shuffle!(y1a, y1b, [0, 2, 4, 6]);
                let y2i = f32x4_shuffle!(y1a, y1b, [1, 3, 5, 7]);
                let z2r = f32x4_shuffle!(z1a, z1b, [0, 2, 4, 6]);
                let z2i = f32x4_shuffle!(z1a, z1b, [1, 3, 5, 7]);
                let w2r = f32x4_shuffle!(w1a, w1b, [0, 2, 4, 6]);
                let w2i = f32x4_shuffle!(w1a, w1b, [1, 3, 5, 7]);

                // perform size-4 FFT
                // TODO; change this to size-4
                let x3r = x2r + z2r; let x3i = x2i + z2i;
                let y3r = y2r + w2r; let y3i = y2i + w2i;
                let z3r = x2r - z2r; let z3i = x2i - z2i;
                let w3r = y2r - w2r; let w3i = y2i - w2i;

                let x4r = x3r + y3r; let x4i = x3i + y3i;
                let z4r = x3r - y3r; let z4i = x3i - y3i;
                let (y4r, y4i, w4r, w4i) = if self.sparams.inverse() {
                    (z3r - w3i, z3i + w3r, z3r + w3i, z3i - w3r)
                } else {
                    (z3r + w3i, z3i - w3r, z3r - w3i, z3i + w3r)
                };

                // apply twiddle factor
                let x5r = x4r;
                let x5i = x4i;
                let y5r = y4r * twiddle1_r - y4i * twiddle1_i;
                let y5i = y4r * twiddle1_i + y4i * twiddle1_r;
                let z5r = z4r * twiddle2_r - z4i * twiddle2_i;
                let z5i = z4r * twiddle2_i + z4i * twiddle2_r;
                let w5r = w4r * twiddle3_r - w4i * twiddle3_i;
                let w5i = w4r * twiddle3_i + w4i * twiddle3_r;

                // convert to rrrr-iiii to riri-riri (unpcklps/unpckups)
                let x6a = f32x4_shuffle!(x5r, x5i, [0, 4, 1, 5]);
                let x6b = f32x4_shuffle!(x5r, x5i, [2, 6, 3, 7]);
                let y6a = f32x4_shuffle!(y5r, y5i, [0, 4, 1, 5]);
                let y6b = f32x4_shuffle!(y5r, y5i, [2, 6, 3, 7]);
                let z6a = f32x4_shuffle!(z5r, z5i, [0, 4, 1, 5]);
                let z6b = f32x4_shuffle!(z5r, z5i, [2, 6, 3, 7]);
                let w6a = f32x4_shuffle!(w5r, w5i, [0, 4, 1, 5]);
                let w6b = f32x4_shuffle!(w5r, w5i, [2, 6, 3, 7]);

                unsafe { *cur1a = x6a }; unsafe { *cur1b = x6b };
                unsafe { *cur2a = y6a }; unsafe { *cur2b = y6b };
                unsafe { *cur3a = z6a }; unsafe { *cur3b = z6b };
                unsafe { *cur4a = w6a }; unsafe { *cur4b = w6b };
            }
        }
    }
}