//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(test)]
extern crate test;
extern crate ysr2_spatializer;

use self::test::Bencher;

use ysr2_spatializer::FdQuant;
use ysr2_spatializer::bandmerger::{BandMerger, Lr4BandMerger};

#[bench]
fn bandmerge_lr4_100000(b: &mut Bencher) {
    let input = vec![FdQuant::new([0.0f32; 8]); 100000];
    let mut output = vec![0.0; input.len()];

    let mut merger = Lr4BandMerger::new(
        &[
            200.0 / 44100.0,
            400.0 / 44100.0,
            800.0 / 44100.0,
            1600.0 / 44100.0,
            3200.0 / 44100.0,
            6400.0 / 44100.0,
            12000.0 / 44100.0,
        ],
    );

    b.iter(move || { merger.merge(&mut output, &input); });
}
