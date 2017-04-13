//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{Kernel, KernelCreationParams, KernelParams, KernelType};

use num_complex::Complex;
use num_traits::{Zero, One};
use num_iter::range_step;

use super::super::{Num, complex_from_slice};


pub fn new_generic_kernel<T : 'static>(cparams: &KernelCreationParams) -> Box<Kernel<T>>
    where T : Num {

    let full_circle = if cparams.inverse { 2 } else { -2 };

    match cparams.kernel_type {
        KernelType::Dit => Box::new(GenericDitKernel {
            cparams: *cparams,
            twiddle_delta: Complex::new(Zero::zero(),
                T::from(cparams.size / cparams.radix / cparams.unit).unwrap() *
                T::from(full_circle).unwrap() * T::PI() / T::from(cparams.size).unwrap()).exp(),
            coef_delta: Complex::new(Zero::zero(),
                T::from(full_circle).unwrap() * T::PI() / T::from(cparams.radix).unwrap()).exp()
        }),
        KernelType::Dif => Box::new(GenericDifKernel {
            cparams: *cparams,
            twiddle_delta: Complex::new(Zero::zero(),
                T::from(cparams.size / cparams.radix / cparams.unit).unwrap() *
                T::from(full_circle).unwrap() * T::PI() / T::from(cparams.size).unwrap()).exp(),
            coef_delta: Complex::new(Zero::zero(),
                T::from(full_circle).unwrap() * T::PI() / T::from(cparams.radix).unwrap()).exp()
        }),
    }
}

#[derive(Debug)]
struct GenericDitKernel<T> {
    cparams: KernelCreationParams,
    twiddle_delta: Complex<T>,
    /// sub-FFT twiddle unit
    coef_delta: Complex<T>,
}

#[derive(Debug)]
struct GenericDifKernel<T> {
    cparams: KernelCreationParams,
    twiddle_delta: Complex<T>,
    /// sub-FFT twiddle unit
    coef_delta: Complex<T>,
}

impl<T> Kernel<T> for GenericDitKernel<T> where T : Num {
    fn transform(&self, params: &mut KernelParams<T>) {
        let cparams = &self.cparams;
        let ref mut data = params.coefs;
        let ref mut wa = params.work_area[0 .. cparams.radix * 2];

        let twiddle_delta = self.twiddle_delta;
        let coef_delta = self.coef_delta;

        for x in range_step(0, cparams.size, cparams.unit * cparams.radix) {
            let mut twiddle_1: Complex<T> = Complex::one();
            for y in 0 .. cparams.unit {
                let mut twiddle_2 = Complex::one();
                for z in 0 .. cparams.radix {
                    let c = complex_from_slice(&data[(x + y + z * cparams.unit) * 2 ..]) *
                        twiddle_2;
                    wa[z * 2    ] = c.re;
                    wa[z * 2 + 1] = c.im;
                    twiddle_2 = twiddle_2 * twiddle_1;
                }
                let mut coef_1 = Complex::one();
                for z in 0 .. cparams.radix {
                    let mut c: Complex<T> = Complex::zero();
                    let mut coef_2 = Complex::one();
                    for w in 0 .. cparams.radix {
                        c = c + coef_2 * complex_from_slice(&wa[w * 2 ..]);
                        coef_2 = coef_2 * coef_1;
                    }
                    data[(x + y + z * cparams.unit) * 2    ] = c.re;
                    data[(x + y + z * cparams.unit) * 2 + 1] = c.im;
                    coef_1 = coef_1 * coef_delta;
                }
                twiddle_1 = twiddle_1 * twiddle_delta;
            }
        }
    }

    fn required_work_area_size(&self) -> usize { self.cparams.radix * 2 }
}

impl<T> Kernel<T> for GenericDifKernel<T> where T : Num {
    fn transform(&self, params: &mut KernelParams<T>) {
        let cparams = &self.cparams;
        let ref mut data = params.coefs;
        let ref mut wa = params.work_area[0 .. cparams.radix * 2];

        let twiddle_delta = self.twiddle_delta;
        let coef_delta = self.coef_delta;

        for x in range_step(0, cparams.size, cparams.unit * cparams.radix) {
            let mut twiddle_1: Complex<T> = Complex::one();
            for y in 0 .. cparams.unit {
                for z in 0 .. cparams.radix {
                    wa[z * 2    ] = data[(x + y + z * cparams.unit) * 2];
                    wa[z * 2 + 1] = data[(x + y + z * cparams.unit) * 2 + 1];
                }
                let mut twiddle_2 = Complex::one();
                let mut coef_1 = Complex::one();
                for z in 0 .. cparams.radix {
                    let mut c: Complex<T> = Complex::zero();
                    let mut coef_2 = twiddle_2;
                    for w in 0 .. cparams.radix {
                        c = c + coef_2 * complex_from_slice(&wa[w * 2 ..]);
                        coef_2 = coef_2 * coef_1;
                    }
                    data[(x + y + z * cparams.unit) * 2    ] = c.re;
                    data[(x + y + z * cparams.unit) * 2 + 1] = c.im;
                    twiddle_2 = twiddle_2 * twiddle_1;
                    coef_1 = coef_1 * coef_delta;
                }
                twiddle_1 = twiddle_1 * twiddle_delta;
            }
        }
    }

    fn required_work_area_size(&self) -> usize { self.cparams.radix * 2 }
}
