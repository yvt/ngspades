//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate test;

use self::test::Bencher;

use biquad;
use utils::assert_num_slice_approx_eq;
use siso::SisoFilter;

#[test]
fn identity() {
    let signal: Vec<_> = (1..256).map(|x| x as f32).collect();
    let coefs = biquad::BiquadCoefs::identity();
    let mut kernel = biquad::SimpleBiquadKernel::new(&coefs, 1);

    let mut signal_new = signal.clone();
    let len = signal.len();
    kernel.render_inplace(&mut [&mut signal_new], 0..len);

    assert_num_slice_approx_eq(&signal_new, &signal, 1.0e-5);
}

#[bench]
fn process_1000000(b: &mut Bencher) {
    let mut signal = vec![0.0; 1000000];
    let coefs = biquad::BiquadCoefs::identity();
    let mut kernel = biquad::SimpleBiquadKernel::new(&coefs, 1);

    b.iter(move || {
        kernel.render_inplace(&mut [&mut signal], 0..1000000);
    });
}
