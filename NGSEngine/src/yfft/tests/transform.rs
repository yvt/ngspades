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

use std::rc::Rc;
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

#[test]
fn fft_correctness_f32() {
    for size_ref in &[1, 2, 4, 8, 16, 32] {
        let size = *size_ref;
        let setup: Setup<f32> = Setup::new(&Options {
            input_data_order: DataOrder::Natural,
            output_data_order: DataOrder::Natural,
            input_data_format: DataFormat::Complex,
            output_data_format: DataFormat::Complex,
            len: size,
            inverse: false
        }).unwrap();
        let mut se = Env::new(&setup);
        let mut result_1 = vec![0f32; size * 2];
        let mut result_2 = vec![0f32; size * 2];
        for pat in test_patterns::<f32>(size) {
            result_1.copy_from_slice(pat.as_slice());
            se.transform(result_1.as_mut_slice());

            naive_dft(pat.as_slice(), result_2.as_mut_slice(), false);

            assert_num_slice_approx_eq(result_1.as_slice(), result_2.as_slice(), 1.0e-4f32);
        }
    }
}
