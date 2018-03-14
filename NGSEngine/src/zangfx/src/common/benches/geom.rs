//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(test)]
extern crate test;
extern crate zangfx_common;
use zangfx_common::*;

struct Xorshift32(u32);

impl Xorshift32 {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

#[bench]
fn pad_u32x3(b: &mut test::Bencher) {
    let mut rng = Xorshift32(1);
    let slices: Vec<Vec<u32>> = (0..1000)
        .map(|_| vec![1; (rng.next() % 3 + 1) as usize])
        .collect();
    let slices: Vec<_> = slices.iter().map(Vec::as_slice).collect();
    let mut results: Vec<[u32; 3]> = slices.iter().map(|_| Default::default()).collect();
    b.iter(move || {
        for (&slice, result) in slices.iter().zip(results.iter_mut()) {
            *result = slice.into_with_pad(5);
        }
    });
}
