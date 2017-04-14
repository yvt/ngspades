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
use super::utils::{StaticParams, StaticParamsConsumer, branch_on_static_params};
use super::super::super::simdutils::{f32x4_bitxor, f32x4_complex_mul_rrii};

use num_complex::Complex;
use num_iter::range_step;

use simd::f32x4;

use std::any::TypeId;
use std::{mem, f32};

pub fn new_x86_sse_kernel<T>(cparams: &KernelCreationParams) -> Option<Box<Kernel<T>>>
    where T : Num {

    // Rust doesn't have partial specialization of generics yet...
    if TypeId::of::<T>() != TypeId::of::<f32>() {
        return None
    }

    match branch_on_static_params(cparams, Factory{}) {
        // This is perfectly safe because we can reach here only when T == f32
        // TODO: move this dirty unsafety somewhere outside
        Some(k) => Some(unsafe{mem::transmute(k)}),
        None => None
    }
}

struct Factory{}
impl StaticParamsConsumer<Option<Box<Kernel<f32>>>> for Factory {
    fn consume<T>(self, cparams: &KernelCreationParams, sparams: T) -> Option<Box<Kernel<f32>>>
        where T : StaticParams {

        match (cparams.radix, cparams.unit) {
            (4, unit) if unit % 4 == 0 => Some(Box::new(SseRadix4Kernel4::new(cparams, sparams))),
            (2, unit) if unit % 4 == 0 => Some(Box::new(SseRadix2Kernel3::new(cparams, sparams))),
            (2, unit) if unit % 2 == 0 => Some(Box::new(SseRadix2Kernel2::new(cparams, sparams))),
            (2, 1)                     => Some(Box::new(SseRadix2Kernel1 { cparams: *cparams })),
            _ => None
        }
    }
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
struct SseRadix2Kernel2<T> {
    cparams: KernelCreationParams,
    twiddles: Vec<f32x4>,
    sparams: T
}

impl<T: StaticParams> SseRadix2Kernel2<T> {
    fn new(cparams: &KernelCreationParams, sparams: T) -> Self {
        sparams.check_param(cparams);
        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit % 2, 0);

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
            sparams: sparams,
        }
    }
}

impl<T: StaticParams> Kernel<f32> for SseRadix2Kernel2<T> {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let sparams = &self.sparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        // TODO: check alignment?

        let twiddles = unsafe { SliceAccessor::new(self.twiddles.as_slice()) };

        let neg_mask_raw: [u32; 4] = [0x80000000, 0x80000000, 0, 0];
        let neg_mask = unsafe { *(&neg_mask_raw as *const u32 as *const f32x4) };

        let pre_twiddle = sparams.kernel_type() == KernelType::Dit;
        let post_twiddle = sparams.kernel_type() == KernelType::Dif;

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
                let x2 = x1;
                let y2 = if pre_twiddle {
                    let t1 = f32x4_shuffle!(y1, y1, [0, 2, 5, 7]); // riri to rrii
                    let t2 = f32x4_complex_mul_rrii(t1, twiddle_1, neg_mask);
                    f32x4_shuffle!(t2, t2, [0, 2, 5, 7]) // rrii to riri
                } else { y1 };

                // perform size-2 FFT
                // (y3a.r, y3a.i, y3b.r, y3b.i)
                let x3 = x2 + y2;
                let y3 = x2 - y2;

                // apply twiddle factor
                // (y1a.r, y1b.r, y1a.i, y1b.i)
                let x4 = x3;
                let y4 = if post_twiddle {
                    let t1 = f32x4_shuffle!(y3, y3, [0, 2, 5, 7]); // riri to rrii
                    let t2 = f32x4_complex_mul_rrii(t1, twiddle_1, neg_mask);
                    f32x4_shuffle!(t2, t2, [0, 2, 5, 7]) // rrii to riri
                } else { y3 };

                unsafe { *cur1 = x4 };
                unsafe { *cur2 = y4 };
            }
        }
    }
}

#[derive(Debug)]
struct SseRadix2Kernel3<T> {
    cparams: KernelCreationParams,
    twiddles: Vec<f32x4>,
    sparams: T,
}

impl<T: StaticParams> SseRadix2Kernel3<T> {
    fn new(cparams: &KernelCreationParams, sparams: T) -> Self {
        sparams.check_param(cparams);
        assert_eq!(cparams.radix, 2);
        assert_eq!(cparams.unit % 4, 0);

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
            sparams: sparams,
        }
    }
}

impl<T: StaticParams> Kernel<f32> for SseRadix2Kernel3<T> {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let sparams = &self.sparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        // TODO: check alignment?

        let twiddles = unsafe { SliceAccessor::new(self.twiddles.as_slice()) };

        let pre_twiddle = sparams.kernel_type() == KernelType::Dit;
        let post_twiddle = sparams.kernel_type() == KernelType::Dif;

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
                let y3r = if pre_twiddle { y2r * twiddle_r - y2i * twiddle_i } else { y2r };
                let y3i = if pre_twiddle { y2r * twiddle_i + y2i * twiddle_r } else { y2i };

                // perform size-2 FFT
                let x4r = x3r + y3r;
                let x4i = x3i + y3i;
                let y4r = x3r - y3r;
                let y4i = x3i - y3i;

                // apply twiddle factor
                let x5r = x4r;
                let x5i = x4i;
                let y5r = if post_twiddle { y4r * twiddle_r - y4i * twiddle_i } else { y4r };
                let y5i = if post_twiddle { y4r * twiddle_i + y4i * twiddle_r } else { y4i };

                // convert to rrrr-iiii to riri-riri (unpcklps/unpckups)
                let x6a = f32x4_shuffle!(x5r, x5i, [0, 4, 1, 5]);
                let x6b = f32x4_shuffle!(x5r, x5i, [2, 6, 3, 7]);
                let y6a = f32x4_shuffle!(y5r, y5i, [0, 4, 1, 5]);
                let y6b = f32x4_shuffle!(y5r, y5i, [2, 6, 3, 7]);

                unsafe { *cur1a = x6a };
                unsafe { *cur1b = x6b };
                unsafe { *cur2a = y6a };
                unsafe { *cur2b = y6b };
            }
        }
    }
}

#[derive(Debug)]
struct SseRadix4Kernel4<T: StaticParams> {
    cparams: KernelCreationParams,
    twiddles: Vec<f32x4>,
    sparams: T
}

impl<T: StaticParams> SseRadix4Kernel4<T> {
    fn new(cparams: &KernelCreationParams, sparams: T) -> Self {
        sparams.check_param(cparams);
        assert_eq!(cparams.radix, 4);
        assert_eq!(cparams.unit % 4, 0);

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

impl<T: StaticParams> Kernel<f32> for SseRadix4Kernel4<T> {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let cparams = &self.cparams;
        let sparams = &self.sparams;
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. cparams.size * 2]) };

        // TODO: check alignment?

        let twiddles = unsafe { SliceAccessor::new(self.twiddles.as_slice()) };
        let pre_twiddle = sparams.kernel_type() == KernelType::Dit;
        let post_twiddle = sparams.kernel_type() == KernelType::Dif;

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
                let y3r = if pre_twiddle { y2r * twiddle1_r - y2i * twiddle1_i } else { y2r };
                let y3i = if pre_twiddle { y2r * twiddle1_i + y2i * twiddle1_r } else { y2i };
                let z3r = if pre_twiddle { z2r * twiddle2_r - z2i * twiddle2_i } else { z2r };
                let z3i = if pre_twiddle { z2r * twiddle2_i + z2i * twiddle2_r } else { z2i };
                let w3r = if pre_twiddle { w2r * twiddle3_r - w2i * twiddle3_i } else { w2r };
                let w3i = if pre_twiddle { w2r * twiddle3_i + w2i * twiddle3_r } else { w2i };

                // perform size-4 FFT
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

                // apply twiddle factor
                let x6r = x5r;
                let x6i = x5i;
                let y6r = if post_twiddle { y5r * twiddle1_r - y5i * twiddle1_i } else { y5r };
                let y6i = if post_twiddle { y5r * twiddle1_i + y5i * twiddle1_r } else { y5i };
                let z6r = if post_twiddle { z5r * twiddle2_r - z5i * twiddle2_i } else { z5r };
                let z6i = if post_twiddle { z5r * twiddle2_i + z5i * twiddle2_r } else { z5i };
                let w6r = if post_twiddle { w5r * twiddle3_r - w5i * twiddle3_i } else { w5r };
                let w6i = if post_twiddle { w5r * twiddle3_i + w5i * twiddle3_r } else { w5i };

                // convert to rrrr-iiii to riri-riri (unpcklps/unpckups)
                let x7a = f32x4_shuffle!(x6r, x6i, [0, 4, 1, 5]);
                let x7b = f32x4_shuffle!(x6r, x6i, [2, 6, 3, 7]);
                let y7a = f32x4_shuffle!(y6r, y6i, [0, 4, 1, 5]);
                let y7b = f32x4_shuffle!(y6r, y6i, [2, 6, 3, 7]);
                let z7a = f32x4_shuffle!(z6r, z6i, [0, 4, 1, 5]);
                let z7b = f32x4_shuffle!(z6r, z6i, [2, 6, 3, 7]);
                let w7a = f32x4_shuffle!(w6r, w6i, [0, 4, 1, 5]);
                let w7b = f32x4_shuffle!(w6r, w6i, [2, 6, 3, 7]);

                unsafe { *cur1a = x7a }; unsafe { *cur1b = x7b };
                unsafe { *cur2a = y7a }; unsafe { *cur2b = y7b };
                unsafe { *cur3a = z7a }; unsafe { *cur3b = z7b };
                unsafe { *cur4a = w7a }; unsafe { *cur4b = w7b };
            }
        }
    }
}