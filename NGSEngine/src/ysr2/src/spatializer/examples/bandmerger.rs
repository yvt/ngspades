//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ysr2_spatializer;
extern crate clap;
extern crate hound;

use ysr2_spatializer::FdQuant;
use ysr2_spatializer::bandmerger::{BandMerger, Lr4BandMerger};
use ysr2_spatializer::rand::{self, Rand};

use std::path::Path;

fn main() {
    use clap::{App, Arg};
    // Use `clap` to parse command-line arguments
    let matches = App::new("bandmerger")
        .author("yvt <i@yvt.jp>")
        .about("verifies the unity gain property of Lr4BandMerger")
        .arg(
            Arg::with_name("OUTPUT")
                .help("Output file name")
                .required(true)
                .index(1),
        )
        .get_matches();

    let mut rng = rand::XorShiftRng::new_unseeded();
    let input: Vec<_> = (0..1000000)
        .map(|i| if i < 100000 {
            FdQuant::new([1.0f32; 8])
        } else if i < 900000 {
            let mut x = FdQuant::new([0.0f32; 8]);
            x.get_mut()[(i - 100000) / 100000] = 1.0;
            x
        } else {
            FdQuant::new([0.0f32; 8])
        } * (<f32>::rand(&mut rng) - 0.5))
        .collect();
    let mut output = vec![0.0; input.len()];

    Lr4BandMerger::new(
        &[
            200.0 / 44100.0,
            400.0 / 44100.0,
            800.0 / 44100.0,
            1600.0 / 44100.0,
            3200.0 / 44100.0,
            6400.0 / 44100.0,
            12000.0 / 44100.0,
        ],
    ).merge(&mut output, &input);

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let path = Path::new(matches.value_of_os("OUTPUT").unwrap());
    let mut writer = hound::WavWriter::create(path, spec).unwrap();

    for &x in output.iter() {
        writer.write_sample(x).unwrap();
    }

    writer.finalize().unwrap();
}
