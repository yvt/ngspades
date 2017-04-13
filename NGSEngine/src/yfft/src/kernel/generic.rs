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


pub fn new_generic_kernel<T>(cparams: &KernelCreationParams) -> Box<Kernel<T>>
    where T : Num {

    match cparams.kernel_type {
        KernelType::Dit => Box::new(GenericDitKernel{ cparams: *cparams }),
        KernelType::Dif => Box::new(GenericDifKernel{ cparams: *cparams }),
    }
}

#[derive(Debug)]
struct GenericDitKernel { cparams: KernelCreationParams }

#[derive(Debug)]
struct GenericDifKernel { cparams: KernelCreationParams }

impl<T> Kernel<T> for GenericDitKernel where T : Num {
    fn transform(&self, params: &mut KernelParams<T>) {
        let cparams = &self.cparams;
        let ref mut data = params.coefs;
        let ref mut wa = params.work_area[0 .. cparams.radix * 2];
        let twiddle_delta: Complex<T> = Complex::new(Zero::zero(),
            T::from(cparams.radix * cparams.unit).unwrap() / T::from(cparams.size).unwrap() *
            T::from(-2).unwrap() * T::PI());

        // TODO: this is all wrong, I guess

        for x in range_step(0, cparams.size, cparams.unit * cparams.radix) {
            let mut twiddle: Complex<T> = Complex::one();
            for y in 0 .. cparams.unit {
                let mut tw2 = Complex::one();
                for z in 0 .. cparams.radix {
                    let c = complex_from_slice(&data[(x + y + z * cparams.unit) * 2 ..]);
                    let c2 = c * tw2;
                    wa[z * 2    ] = c2.re;
                    wa[z * 2 + 1] = c2.im;
                    tw2 = tw2 * twiddle;
                }
                for z in 0 .. cparams.radix {
                    let mut c: Complex<T> = Complex::zero();
                    for w in 0 .. cparams.radix {
                        let f = Complex::new(Zero::zero(),
                            T::from(z * w).unwrap() / T::from(cparams.radix).unwrap() *
                            T::from(-2).unwrap() * T::PI());
                        c = c + f * complex_from_slice(&wa[w * 2 ..]);
                    }
                    data[(x + y + z * cparams.unit) * 2    ] = c.re;
                    data[(x + y + z * cparams.unit) * 2 + 1] = c.im;
                }
                twiddle = twiddle * twiddle_delta;
            }
        }
    }

    fn required_work_area_size(&self) -> usize { self.cparams.radix * 2 }
}

impl<T> Kernel<T> for GenericDifKernel where T : Num {
    fn transform(&self, params: &mut KernelParams<T>) {
        let cparams = &self.cparams;
        let ref mut data = params.coefs;
        let ref mut wa = params.work_area[0 .. cparams.radix * 2];

        let full_circle = if cparams.inverse { 2 } else { -2 };

        let twiddle_delta: Complex<T> = Complex::new(Zero::zero(),
            T::from(cparams.size / cparams.radix / cparams.unit).unwrap() *
            T::from(full_circle).unwrap() * T::PI() / T::from(cparams.size).unwrap()).exp();

        // sub-FFT twiddle unit
        let coef_delta = Complex::new(Zero::zero(),
            T::from(full_circle).unwrap() * T::PI() / T::from(cparams.radix).unwrap()).exp();

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
