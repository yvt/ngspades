//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

//! Defines generic FFT kernels optimized for certain known radix values, but without any specific processor or
//! instruction set specific optimizations.
//!
//! Performances
//! ------------
//!
//! According to a benchmark result, this kernel runs about 10x slower than a commercial-level FFT library on a Skylake
//! machine.

use super::{Kernel, KernelCreationParams, KernelParams, KernelType};

use num_complex::Complex;
use num_traits::{Zero, One};
use num_iter::range_step;

use super::super::{Num, complex_from_slice, mul_neg_i, mul_pos_i};

use std::fmt::Debug;
use std::marker::PhantomData;

pub fn new_specialized_generic_kernel<T>(cparams: &KernelCreationParams) -> Option<Box<Kernel<T>>>
    where T : Num {
    match (cparams.radix, cparams.inverse) {
        (2, _) => Some(new_specialized_generic_kernel_inner::<T, SmallFFT2<T>>(cparams)),
        (4, false) => Some(new_specialized_generic_kernel_inner::<T, SmallForwardFFT4<T>>(cparams)),
        (4, true) => Some(new_specialized_generic_kernel_inner::<T, SmallBackwardFFT4<T>>(cparams)),
        _ => None
    }
}

fn new_specialized_generic_kernel_inner<T, TSmallFFT>(cparams: &KernelCreationParams) -> Box<Kernel<T>>
    where T : Num,
          TSmallFFT : SmallFFT<T> {

    let full_circle = if cparams.inverse { 2 } else { -2 };

    match cparams.kernel_type {
        KernelType::Dit => Box::new(SpecializedGenericDitKernel::<T, TSmallFFT> {
            cparams: *cparams,
            twiddle_delta: Complex::new(Zero::zero(),
                T::from(cparams.size / cparams.radix / cparams.unit).unwrap() *
                T::from(full_circle).unwrap() * T::PI() / T::from(cparams.size).unwrap()).exp(),
            small_fft: PhantomData,
        }),
        KernelType::Dif => Box::new(SpecializedGenericDifKernel::<T, TSmallFFT> {
            cparams: *cparams,
            twiddle_delta: Complex::new(Zero::zero(),
                T::from(cparams.size / cparams.radix / cparams.unit).unwrap() *
                T::from(full_circle).unwrap() * T::PI() / T::from(cparams.size).unwrap()).exp(),
            small_fft: PhantomData,
        }),
    }
}

trait SmallFFT<T> : Debug + Default + 'static {
    fn radix() -> usize;
    fn load(&mut self, data: &[T], offset: usize, stride: usize);
    fn twiddle(&mut self, c: Complex<T>);
    fn transform(&mut self);
    fn store(&self, data: &mut[T], offset: usize, stride: usize);
}

#[derive(Debug, Clone, Copy, Default)]
struct SmallFFT2<T> {
    x1: Complex<T>,
    x2: Complex<T>,
}

impl<T: Num> SmallFFT<T> for SmallFFT2<T> {
    #[inline] fn radix() -> usize { 2 }
    #[inline] fn load(&mut self, data: &[T], offset: usize, stride: usize) {
        self.x1.re = data[offset];
        self.x1.im = data[offset + 1];
        self.x2.re = data[offset + stride];
        self.x2.im = data[offset + stride + 1];
    }
    #[inline] fn twiddle(&mut self, c: Complex<T>) {
        self.x2 = self.x2 * c;
    }
    #[inline] fn transform(&mut self) {
        let orig = *self;
        self.x1 = orig.x1 + orig.x2;
        self.x2 = orig.x1 - orig.x2;
    }
    #[inline] fn store(&self, data: &mut[T], offset: usize, stride: usize) {
        data[offset] = self.x1.re;
        data[offset + 1] = self.x1.im;
        data[offset + stride] = self.x2.re;
        data[offset + stride + 1] = self.x2.im;
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SmallForwardFFT4<T> {
    x1: Complex<T>,
    x2: Complex<T>,
    x3: Complex<T>,
    x4: Complex<T>,
}

impl<T: Num> SmallFFT<T> for SmallForwardFFT4<T> {
    #[inline] fn radix() -> usize { 4 }
    #[inline] fn load(&mut self, data: &[T], offset: usize, stride: usize) {
        self.x1.re = data[offset];
        self.x1.im = data[offset + 1];
        self.x2.re = data[offset + stride];
        self.x2.im = data[offset + stride + 1];
        self.x3.re = data[offset + stride * 2];
        self.x3.im = data[offset + stride * 2 + 1];
        self.x4.re = data[offset + stride * 3];
        self.x4.im = data[offset + stride * 3 + 1];
    }
    #[inline] fn twiddle(&mut self, c: Complex<T>) {
        let c2 = c * c;
        self.x2 = self.x2 * c;
        self.x3 = self.x3 * c2;
        self.x4 = self.x4 * (c * c2);
    }
    #[inline] fn transform(&mut self) {
        let t1 = self.x1 + self.x3;
        let t2 = self.x2 + self.x4;
        let t3 = self.x1 - self.x3;
        let t4 = self.x2 - self.x4;
        self.x1 = t1 + t2;
        self.x2 = t3 - mul_pos_i(t4);
        self.x3 = t1 - t2;
        self.x4 = t3 + mul_pos_i(t4);
    }
    #[inline] fn store(&self, data: &mut[T], offset: usize, stride: usize) {
        data[offset] = self.x1.re;
        data[offset + 1] = self.x1.im;
        data[offset + stride] = self.x2.re;
        data[offset + stride + 1] = self.x2.im;
        data[offset + stride * 2] = self.x3.re;
        data[offset + stride * 2 + 1] = self.x3.im;
        data[offset + stride * 3] = self.x4.re;
        data[offset + stride * 3 + 1] = self.x4.im;
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SmallBackwardFFT4<T> {
    x1: Complex<T>,
    x2: Complex<T>,
    x3: Complex<T>,
    x4: Complex<T>,
}

impl<T: Num> SmallFFT<T> for SmallBackwardFFT4<T> {
    #[inline] fn radix() -> usize { 4 }
    #[inline] fn load(&mut self, data: &[T], offset: usize, stride: usize) {
        self.x1.re = data[offset];
        self.x1.im = data[offset + 1];
        self.x2.re = data[offset + stride];
        self.x2.im = data[offset + stride + 1];
        self.x3.re = data[offset + stride * 2];
        self.x3.im = data[offset + stride * 2 + 1];
        self.x4.re = data[offset + stride * 3];
        self.x4.im = data[offset + stride * 3 + 1];
    }
    #[inline] fn twiddle(&mut self, c: Complex<T>) {
        let c2 = c * c;
        self.x2 = self.x2 * c;
        self.x3 = self.x3 * c2;
        self.x4 = self.x4 * (c * c2);
    }
    #[inline] fn transform(&mut self) {
        let t1 = self.x1 + self.x3;
        let t2 = self.x2 + self.x4;
        let t3 = self.x1 - self.x3;
        let t4 = self.x2 - self.x4;
        self.x1 = t1 + t2;
        self.x2 = t3 + mul_pos_i(t4);
        self.x3 = t1 - t2;
        self.x4 = t3 - mul_pos_i(t4);
    }
    #[inline] fn store(&self, data: &mut[T], offset: usize, stride: usize) {
        data[offset] = self.x1.re;
        data[offset + 1] = self.x1.im;
        data[offset + stride] = self.x2.re;
        data[offset + stride + 1] = self.x2.im;
        data[offset + stride * 2] = self.x3.re;
        data[offset + stride * 2 + 1] = self.x3.im;
        data[offset + stride * 3] = self.x4.re;
        data[offset + stride * 3 + 1] = self.x4.im;
    }
}

#[derive(Debug)]
struct SpecializedGenericDitKernel<T, TSmallFFT> {
    cparams: KernelCreationParams,
    twiddle_delta: Complex<T>,
    small_fft: PhantomData<TSmallFFT>,
}

#[derive(Debug)]
struct SpecializedGenericDifKernel<T, TSmallFFT> {
    cparams: KernelCreationParams,
    twiddle_delta: Complex<T>,
    small_fft: PhantomData<TSmallFFT>,
}

impl<T, TSmallFFT> Kernel<T> for SpecializedGenericDitKernel<T, TSmallFFT>
    where T : Num,
          TSmallFFT : SmallFFT<T> {

    fn transform(&self, params: &mut KernelParams<T>) {
        let cparams = &self.cparams;
        let ref mut data = params.coefs[0 .. cparams.size * 2];

        let twiddle_delta = self.twiddle_delta;
        let mut small_fft = TSmallFFT::default();

        let radix = TSmallFFT::radix();
        assert_eq!(TSmallFFT::radix(), cparams.radix);

        for x in range_step(0, cparams.size, cparams.unit * radix) {
            let mut twiddle_1: Complex<T> = Complex::one();
            for y in 0 .. cparams.unit {
                small_fft.load(data, (x + y) * 2, cparams.unit * 2);
                small_fft.twiddle(twiddle_1);
                small_fft.transform();
                small_fft.store(data, (x + y) * 2, cparams.unit * 2);
                twiddle_1 = twiddle_1 * twiddle_delta;
            }
        }
    }
}

impl<T, TSmallFFT> Kernel<T> for SpecializedGenericDifKernel<T, TSmallFFT>
    where T : Num,
          TSmallFFT : SmallFFT<T>  {

    fn transform(&self, params: &mut KernelParams<T>) {
        let cparams = &self.cparams;
        let ref mut data = params.coefs[0 .. cparams.size * 2];

        let twiddle_delta = self.twiddle_delta;
        let mut small_fft = TSmallFFT::default();

        let radix = TSmallFFT::radix();
        assert_eq!(TSmallFFT::radix(), cparams.radix);

        for x in range_step(0, cparams.size, cparams.unit * radix) {
            let mut twiddle_1: Complex<T> = Complex::one();
            for y in 0 .. cparams.unit {
                small_fft.load(data, (x + y) * 2, cparams.unit * 2);
                small_fft.transform();
                small_fft.twiddle(twiddle_1);
                small_fft.store(data, (x + y) * 2, cparams.unit * 2);
                twiddle_1 = twiddle_1 * twiddle_delta;
            }
        }
    }
}