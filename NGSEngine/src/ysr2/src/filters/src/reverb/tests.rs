//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate test;

use self::test::Bencher;

use reverb;
use Filter;

#[bench]
fn process_1000000(b: &mut Bencher) {
    let mut signal = vec![0.0; 1000000];
    let mut kernel = reverb::MatrixReverb::new(&reverb::MatrixReverbParams {
        reverb_time: 100000.0,
        mean_delay_time: 5000.0,
        diffusion: 0.8,
        reverb_time_hf_ratio: 0.8,
        high_frequency_ref: 0.3,
    });

    b.iter(move || {
        kernel.render_inplace(&mut [&mut signal], 0..1000000);
    });
}
