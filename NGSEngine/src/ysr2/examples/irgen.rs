//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Outputs an impulse response of an artifical reverb filter.
extern crate ysr2;
extern crate clap;
extern crate hound;

use std::path::Path;
use std::str::FromStr;

use ysr2::filters::reverb;

fn main() {
    use clap::{App, Arg};
    // Use `clap` to parse command-line arguments
    let matches = App::new("irgen")
        .author("yvt <i@yvt.jp>")
        .about("generates artifical reverb IR using YSR2")
        .arg(
            Arg::with_name("OUTPUT")
                .help("Output file name")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("algorithm")
                .short("a")
                .long("algorithm")
                .value_name("NAME")
                .help("Specifies the reverb algorithm")
                .takes_value(true)
                .possible_values(&["matrix"])
                .default_value("matrix"),
        )
        .arg(
            Arg::with_name("rt60")
                .short("t")
                .long("rt60")
                .value_name("SECONDS")
                .help("Specifies the RT60 decay time in seconds")
                .takes_value(true)
                .default_value("2"),
        )
        .arg(
            Arg::with_name("meanfreedelay")
                .short("m")
                .long("meanfreedelay")
                .value_name("SECONDS")
                .help("Specifies the mean free path delay in seconds")
                .takes_value(true)
                .default_value("0.05"),
        )
        .arg(
            Arg::with_name("sampling_rate")
                .short("r")
                .long("rate")
                .value_name("RATE")
                .help("Specifies the sampling rate in hertz")
                .takes_value(true)
                .default_value("44100"),
        )
        .arg(
            Arg::with_name("channels")
                .short("c")
                .long("channels")
                .value_name("CHANNELS")
                .help("Specifies the number of channels")
                .takes_value(true)
                .default_value("2"),
        )
        .get_matches();
    let sampling_rate = f64::from_str(matches.value_of("sampling_rate").unwrap()).unwrap();
    let rt60 = f64::from_str(matches.value_of("rt60").unwrap()).unwrap();
    let num_channels = usize::from_str(matches.value_of("channels").unwrap()).unwrap();
    let meanfreedelay = f64::from_str(matches.value_of("meanfreedelay").unwrap()).unwrap();

    if !sampling_rate.is_finite() || sampling_rate < 1.0 || sampling_rate > 4.0e9 {
        panic!("The specified sampling rate is out of range.");
    }
    if !rt60.is_finite() || rt60 <= 0.0 {
        panic!("The specified decay time is out of range.");
    }
    if !meanfreedelay.is_finite() || meanfreedelay <= 0.0 {
        panic!("The specified mean free path delay is out of range.");
    }
    if num_channels < 1 || num_channels > 8 {
        panic!("The specified number of channels is out of range.");
    }

    let mut remaining_samples = (rt60 * 2.0 * sampling_rate) as u64;

    let mut input_buffer = vec![0.0; 1024];
    let mut output_buffer = vec![vec![0.0; 1024]; num_channels];

    let algorithm = matches.value_of("algorithm").unwrap();
    let mut filter: Box<ysr2::filters::Filter> = if algorithm == "matrix" {
        Box::new(reverb::MatrixReverb::new(&reverb::MatrixReverbParams {
            reverb_time: rt60 * sampling_rate,
            mean_delay_time: meanfreedelay * sampling_rate,
            diffusion: 1.0,
            reverb_time_hf_ratio: 0.5,
            high_frequency_ref: (5000.0 / sampling_rate).min(0.4),
        }))
    } else {
        unreachable!()
    };

    let spec = hound::WavSpec {
        channels: num_channels as u16,
        sample_rate: sampling_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let path = Path::new(matches.value_of_os("OUTPUT").unwrap());
    let mut writer = hound::WavWriter::create(path, spec).unwrap();

    input_buffer[0] = 1.0;

    while remaining_samples > 0 {
        let num_samples = if remaining_samples > 1024 {
            1024
        } else {
            remaining_samples as usize
        };

        let mut output_slices: Vec<_> = output_buffer.iter_mut().map(|x| &mut x[..]).collect();
        filter.render(
            &mut output_slices,
            0..num_samples,
            Some((&[&input_buffer], 0..num_samples)),
        );
        input_buffer[0] = 0.0;

        for i in 0..num_samples {
            for k in 0..num_channels {
                writer.write_sample(output_slices[k][i]).unwrap();
            }
        }

        remaining_samples -= num_samples as u64;
    }

    writer.finalize().unwrap();
}
