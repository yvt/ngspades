//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate test;

use self::test::Bencher;
use std::borrow::Borrow;
use std::cmp::min;
use std::ops::Range;

use ysr2_common::stream::Generator;
use ysr2_common::dispatch::SerialQueue;
use conv::{IrSpectrum, MultiConvolver, ConvParams, ConvSetup};

use utils::assert_num_slice_approx_eq;

struct Player<T> {
    data: T,
    offset: usize,
}

impl<T> Player<T> {
    fn new(data: T) -> Self {
        Self { data, offset: 0 }
    }
}

impl<T: Borrow<[f32]>> Generator for Player<T> {
    fn render(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        assert_eq!(to.len(), 1);

        let slice = self.data.borrow();
        let out_slice = &mut to[0][range.clone()];
        let num_processed = min(out_slice.len(), slice.len().saturating_sub(self.offset));
        out_slice[0..num_processed].copy_from_slice(
            &slice[self.offset..
                       self.offset +
                           num_processed],
        );
        for x in out_slice[num_processed..].iter_mut() {
            *x = 0.0;
        }

        self.offset += num_processed;
    }

    fn skip(&mut self, num_samples: usize) {
        self.offset += num_samples;
    }

    fn is_active(&self) -> bool {
        self.offset < self.data.borrow().len()
    }
}

fn test_patterns() -> Vec<Vec<f32>> {
    let mut vec = Vec::new();
    for x in 1..8 {
        let size = x * 2;
        let mut vec2 = vec![0.0; size];
        vec2[size - 1] = 1.0;
        vec.push(vec2);

        vec.push((0..size).map(|x| ((x * 3 + 7) & 0xf) as f32).collect());
        vec.push(
            (0..size)
                .map(|x| ((x * 3 + 7) ^ (x * 7 + 3) ^ (x >> 1)) as f32)
                .collect(),
        );
    }

    vec
}

fn naive_conv(out: &mut [f32], x: &[f32], y: &[f32]) {
    for v in out.iter_mut() {
        *v = 0.0;
    }
    for (i, &y) in y.iter().enumerate() {
        for (k, &x) in x.iter().enumerate() {
            out[i + k] += x * y;
        }
    }
}

fn test_solo_with_patterns(setup: &ConvSetup, pat1: &[f32], pat2: &[f32]) {
    let ir = IrSpectrum::from_ir(pat2, setup);
    let mut conv = MultiConvolver::new(setup, 1, SerialQueue);
    let src = conv.insert_source(Player::new(pat1));
    conv.insert_mapping(&src, &ir, 0).unwrap();

    let latency = setup.params().latency;
    let mut out_buf = vec![0.0; (pat1.len() + pat2.len() + latency) * 2];
    let mut ref_buf = out_buf.clone();

    println!("  test_solo_with_patterns");
    println!("    signal = {:?}", pat1);
    println!("    ir = {:?}", pat2);
    println!("    ir (fftd) = {:?}", &ir);

    naive_conv(&mut ref_buf[latency..], pat1, pat2);

    conv.render(&mut [&mut out_buf[..]], 0..ref_buf.len());

    assert_num_slice_approx_eq(&out_buf, &ref_buf, 1.0e-5);
}

fn test_solo_with_params(params: &ConvParams) {
    let patterns = test_patterns();
    let setup = ConvSetup::new(params);

    println!("test_solo_with_params");
    println!("  setup = {:?}", setup);

    for pat1 in patterns.iter() {
        for pat2 in patterns.iter() {
            test_solo_with_patterns(&setup, pat1, pat2);
        }
    }
}

#[test]
fn conv_solo_simple1() {
    test_solo_with_params(&ConvParams {
        // (2^3) * 4
        blocks: vec![(3, 4)],
        latency: 8,
    });
}

#[test]
fn conv_solo_simple2() {
    test_solo_with_params(&ConvParams {
        // (2^3) * 2
        blocks: vec![(3, 2)],
        latency: 8,
    });
}

#[test]
fn conv_solo_simple3() {
    test_solo_with_params(&ConvParams {
        // (2^2) * 8
        blocks: vec![(2, 8)],
        latency: 4,
    });
}

#[test]
fn conv_solo_simple4() {
    test_solo_with_params(&ConvParams {
        // (2^1) * 16
        blocks: vec![(1, 16)],
        latency: 2,
    });
}

#[test]
fn conv_solo_simple_latency_1() {
    test_solo_with_params(&ConvParams {
        // (2^3) * 4
        blocks: vec![(3, 4)],
        latency: 16,
    });
}

#[test]
fn conv_solo_simple_latency_2() {
    test_solo_with_params(&ConvParams {
        // (2^3) * 4
        blocks: vec![(3, 4)],
        latency: 24,
    });
}

#[test]
fn conv_solo_nonuniform1() {
    test_solo_with_params(&ConvParams {
        blocks: vec![
            // (2^1) * 1
            (1, 1),
            // (2^2) * 4
            (2, 4),
        ],
        latency: 2,
    });
}

#[test]
fn conv_solo_nonuniform2() {
    test_solo_with_params(&ConvParams {
        blocks: vec![
            // (2^1) * 3
            (1, 3),
            // (2^2) * 4
            (2, 4),
        ],
        latency: 2,
    });
}

#[test]
fn conv_solo_nonuniform3() {
    test_solo_with_params(&ConvParams {
        blocks: vec![
            // (2^1) * 5
            (1, 5),
            // (2^2) * 4
            (2, 4),
        ],
        latency: 2,
    });
}

#[test]
fn conv_solo_nonuniform4() {
    test_solo_with_params(&ConvParams {
        blocks: vec![
            // (2^1) * 1
            (1, 1),
            // (2^2) * 1
            (2, 1),
            // (2^3) * 4
            (3, 4),
        ],
        latency: 2,
    });
}
