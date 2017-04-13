//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(test)]

extern crate yfft;
extern crate test;

use test::Bencher;
use std::rc::Rc;
use yfft::*;

#[bench]
fn simple_benchmark(b: &mut Bencher) {
    let setup: Setup<f32> = Setup::new(&Options {
        input_data_order: DataOrder::Natural,
        output_data_order: DataOrder::Swizzled,
        input_data_format: DataFormat::Complex,
        output_data_format: DataFormat::Complex,
        len: 256,
        inverse: false
    }).unwrap();
    let mut senv = Env::new(&setup);
    let mut buf = vec![0f32; 512];
    b.iter(move || {
        senv.transform(buf.as_mut_slice());
    })
}
