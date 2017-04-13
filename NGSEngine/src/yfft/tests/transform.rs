//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate yfft;
extern crate num_complex;
extern crate num_traits;

use num_complex::Complex;
use num_traits::{Zero, One};

use yfft::*;

fn naive_dft<T : yfft::Num>(input: &[T], output: &mut[T], inverse: bool) {
    let len = input.len() / 2;
    let full_circle = if inverse { 2 } else { -2 };
    let twiddle_delta: Complex<T> = Complex::new(Zero::zero(),
        T::from(full_circle).unwrap() * T::PI() / T::from(len).unwrap()).exp();
    let mut twiddle_1 = Complex::one();
    for x in 0 .. len {
        let mut twiddle_2 = Complex::one();
        let mut sum = Complex::zero();

        for y in 0 .. len {
            sum = sum + Complex::new(input[y * 2], input[y * 2 + 1]) * twiddle_2;
            twiddle_2 = twiddle_2 * twiddle_1;
        }

        output[x * 2] = sum.re;
        output[x * 2 + 1] = sum.im;

        twiddle_1 = twiddle_1 * twiddle_delta;
    }
}

fn assert_num_slice_approx_eq<T : yfft::Num>(got: &[T], expected: &[T], eps: T) {
    assert_eq!(got.len(), expected.len());
    for i in 0 .. got.len() {
        let a = got[i];
        let b = expected[i];
        if (a - b).abs() > eps {
            assert!((a - b).abs() < eps,
                "assertion failed: `got almost equal to expected` \
                    (got: `{:?}`, expected: `{:?}`)", got, expected);
        }
    }
}

// thanks to the linearity of DFT, we only need as many test cases as the DFT size
// (unless some buggy code breaks it)
fn test_patterns<T : yfft::Num>(size: usize) -> Vec<Vec<T>> {
    let mut vec = Vec::new();
    vec.push(vec![T::zero(); size * 2]);
    for x in 0 .. size {
        let mut vec2 = vec![T::zero(); size * 2];
        vec2[x * 2] = One::one();
        vec.push(vec2);
    }
    for x in 0 .. size {
        let mut vec2 = vec![T::zero(); size * 2];
        vec2[x * 2 + 1] = One::one();
        vec.push(vec2);
    }
    vec
}

fn simple_fft<T : Num>(inverse: bool) {
    for size_ref in &[1, 2, 3, 4, 5, 6, 7, 8, 16, 32, 40, 49] {
        let size = *size_ref;
        let setup: Setup<T> = Setup::new(&Options {
            input_data_order: DataOrder::Natural,
            output_data_order: DataOrder::Natural,
            input_data_format: DataFormat::Complex,
            output_data_format: DataFormat::Complex,
            len: size,
            inverse: inverse
        }).unwrap();
        let mut se = Env::new(&setup);
        let mut result_1 = vec![T::zero(); size * 2];
        let mut result_2 = vec![T::zero(); size * 2];
        for pat in test_patterns::<T>(size) {
            result_1.copy_from_slice(pat.as_slice());
            se.transform(result_1.as_mut_slice());

            naive_dft(pat.as_slice(), result_2.as_mut_slice(), inverse);

            assert_num_slice_approx_eq(result_1.as_slice(), result_2.as_slice(), T::from(1.0e-4).unwrap());
        }
    }
}

#[test]
fn fft_forward_f32() { simple_fft::<f32>(false); }

#[test]
fn fft_forward_f64() { simple_fft::<f64>(false); }

#[test]
fn fft_backward_f32() { simple_fft::<f32>(true); }

#[test]
fn fft_backward_f64() { simple_fft::<f64>(true); }

fn fft_roundtrip_shortcut<T : Num>() {
    for size_ref in &[1, 2, 3, 4, 5, 6, 7, 8, 16, 32, 40, 49] {
        let size = *size_ref;

        let setup1: Setup<T> = Setup::new(&Options {
            input_data_order: DataOrder::Natural,
            output_data_order: DataOrder::Swizzled,
            input_data_format: DataFormat::Complex,
            output_data_format: DataFormat::Complex,
            len: size,
            inverse: false
        }).unwrap();
        let setup2: Setup<T> = Setup::new(&Options {
            input_data_order: DataOrder::Swizzled,
            output_data_order: DataOrder::Natural,
            input_data_format: DataFormat::Complex,
            output_data_format: DataFormat::Complex,
            len: size,
            inverse: true
        }).unwrap();

        let mut env1 = Env::new(&setup1);
        let mut env2 = Env::new(&setup2);

        let factor = T::one() / T::from(size).unwrap();

        let mut result = vec![T::zero(); size * 2];
        for pat in test_patterns::<T>(size) {
            result.copy_from_slice(pat.as_slice());
            env1.transform(result.as_mut_slice());
            env2.transform(result.as_mut_slice());

            for e in &mut result {
                *e = *e * factor;
            }

            assert_num_slice_approx_eq(result.as_slice(), pat.as_slice(), T::from(1.0e-4).unwrap());
        }
    }
}

#[test]
fn fft_roundtrip_shortcut_f32() { fft_roundtrip_shortcut::<f32>(); }

#[test]
fn fft_roundtrip_shortcut_f64() { fft_roundtrip_shortcut::<f64>(); }
