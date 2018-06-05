//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Outputs an impulse response of an artifical reverb filter.
extern crate ysr2;
extern crate clap;
extern crate hound;
extern crate ngsterrain;

use std::path::Path;
use std::str::FromStr;

use ysr2::filters::reverb;

fn main() {
    use clap::{App, Arg, SubCommand, AppSettings};
    // Use `clap` to parse command-line arguments
    let matches = App::new("irgen")
        .author("yvt <i@yvt.jp>")
        .about("generates artifical reverb IR using YSR2")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("model")
                .about("use classic reverb algorithms")
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
                    Arg::with_name("OUTPUT")
                        .help("Output file name")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("simulate")
                .about("use sound propagation simulation")
                .arg(
                    Arg::with_name("algorithm")
                        .short("a")
                        .long("algorithm")
                        .value_name("NAME")
                        .help("Specifies the simulation algorithm")
                        .takes_value(true)
                        .possible_values(&["ptdr"])
                        .default_value("ptdr"),
                )
                .arg(
                    Arg::with_name("GEOMETRY")
                        .short("g")
                        .long("geometry")
                        .value_name("FILENAME.vox")
                        .help("Specifies the geometry")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("num_rays")
                        .short("n")
                        .long("rays")
                        .value_name("SAMPLES")
                        .help("Specifies the number of rays")
                        .takes_value(true)
                        .default_value("1000000"),
                )
                .arg(
                    Arg::with_name("max_bounces")
                        .short("m")
                        .long("bounces")
                        .value_name("BOUNCES")
                        .help("Specifies the maximum number of bounces")
                        .takes_value(true)
                        .default_value("128"),
                )
                .arg(
                    Arg::with_name("skip_bandmerge")
                        .long("skip-bandmerge")
                        .help("Skip the band merging (and outputs IR for each individual frequency band)"),
                )
                .arg(
                    Arg::with_name("OUTPUT")
                        .help("Output file name")
                        .required(true)
                        .index(1),
                ),
        )
        .arg(
            Arg::with_name("direct")
                .short("d")
                .long("direct")
                .help("Include the direct path in the output")
                .global(true),
        )
        .arg(
            Arg::with_name("sampling_rate")
                .short("r")
                .long("rate")
                .value_name("RATE")
                .help("Specifies the sampling rate in hertz")
                .takes_value(true)
                .global(true)
                .default_value("44100"),
        )
        .arg(
            Arg::with_name("channels")
                .short("c")
                .long("channels")
                .value_name("CHANNELS")
                .help("Specifies the number of channels")
                .takes_value(true)
                .global(true)
                .default_value("2"),
        )
        .get_matches();
    let sampling_rate = f64::from_str(matches.value_of("sampling_rate").unwrap()).unwrap();
    let num_channels = usize::from_str(matches.value_of("channels").unwrap()).unwrap();

    if !sampling_rate.is_finite() || sampling_rate < 1.0 || sampling_rate > 4.0e9 {
        panic!("The specified sampling rate is out of range.");
    }
    if num_channels < 1 || num_channels > 8 {
        panic!("The specified number of channels is out of range.");
    }

    let include_direct = matches.is_present("direct");

    let mut output: Vec<Vec<f32>> = vec![Vec::new(); num_channels];

    let sc_matches = if let Some(matches) = matches.subcommand_matches("model") {
        let rt60 = f64::from_str(matches.value_of("rt60").unwrap()).unwrap();
        let meanfreedelay = f64::from_str(matches.value_of("meanfreedelay").unwrap()).unwrap();

        if !rt60.is_finite() || rt60 <= 0.0 {
            panic!("The specified decay time is out of range.");
        }
        if !meanfreedelay.is_finite() || meanfreedelay <= 0.0 {
            panic!("The specified mean free path delay is out of range.");
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

            for i in 0..num_channels {
                output[i].extend(output_slices[i].iter());
            }

            remaining_samples -= num_samples as u64;
        }

        if include_direct {
            for output in output.iter_mut() {
                output[0] += 1.0;
            }
        }

        matches
    } else if let Some(matches) = matches.subcommand_matches("simulate") {
        use ysr2::spatializer;
        use ysr2::spatializer::{FdQuant, Raytracer, ngster, flattener, rand};
        use ysr2::spatializer::cgmath::{vec3, Vector3};
        use ysr2::spatializer::cgmath::prelude::*;
        use ysr2::spatializer::flattener::Flattener;

        // Load the geometry
        use std::fs::File;
        use std::io::BufReader;
        let input_low = matches.value_of("GEOMETRY").unwrap().to_lowercase();
        let input_path = matches.value_of_os("GEOMETRY").unwrap();
        let file = File::open(input_path).unwrap();
        let mut reader = BufReader::new(file);
        let terrain: ngsterrain::Terrain = if input_low.ends_with(".vxl") {
            ngsterrain::io::from_voxlap_vxl(vec3(512, 512, 64), &mut reader).unwrap()
        } else {
            ngsterrain::io::from_magicavoxel(&mut reader).unwrap()
        };

        if num_channels != 2 {
            panic!("The number of channel != 2 is not supported yet.");
        }

        if sampling_rate < 24000.0 {
            panic!("The sampling rate must be at least 24000.");
        }

        let num_samples = (sampling_rate * 4.0) as usize;

        // Locate the source and listener
        let size = terrain.size().cast::<f32>().unwrap();
        let mut listener_pos = vec3(size.x * 0.4, size.y * 0.4, size.z);
        let mut source_pos = vec3(size.x * 0.4, size.y * 0.6, size.z);

        while source_pos.z > 4.0 {
            let mut new_pos = source_pos;
            new_pos.z -= 2.0;
            if terrain.get_voxel(new_pos.cast().unwrap()).is_some() {
                break;
            }
            source_pos = new_pos;
        }
        source_pos.z += 4.0;

        while listener_pos.z > 4.0 {
            let mut new_pos = listener_pos;
            new_pos.z -= 2.0;
            if terrain.get_voxel(new_pos.cast().unwrap()).is_some() {
                break;
            }
            listener_pos = new_pos;
        }
        listener_pos.z += 4.0;

        println!("listener position = {:?}", listener_pos);
        println!("source position = {:?}", source_pos);

        // Set the world up
        struct StereoMcChannelMapper;
        impl flattener::McChannelMapper<f32> for StereoMcChannelMapper {
            fn num_channels(&self) -> usize {
                2
            }

            fn map(&self, direction: Vector3<f32>, out: &mut [f32]) {
                // equal-power panning
                let dir = direction.normalize();
                let x = dir.x * 0.5;
                out[0] = (0.5 - x).sqrt();
                out[1] = (0.5 + x).sqrt();
            }
        }
        let world = spatializer::World {
            absorption: FdQuant::new(
                [
                    0.1e-3,
                    0.3e-3,
                    0.6e-3,
                    1.0e-3,
                    1.9e-3,
                    5.8e-3,
                    20.3e-3,
                    30.0e-3,
                ],
            ),
            speed_of_sound: (340.0 / sampling_rate) as f32,
        };
        let mat_map = ngster::ConstantMaterialMap::new(spatializer::Material {
            scatter: FdQuant::new([0.1; 8]),
            absorption: FdQuant::new([0.02, 0.02, 0.03, 0.03, 0.04, 0.05, 0.05, 0.06]),
        });
        let mut tracer = ngster::TerrainRaytracer::new(&terrain, mat_map, 1.0);
        let bli_source = flattener::Lanczos4BliSource::<f32>::new();
        let ch_mapper = StereoMcChannelMapper;
        let mut flt = flattener::McFlattener::new(bli_source, ch_mapper, num_samples);

        let mut rng = rand::XorShiftRng::new_unseeded();

        let algorithm = matches.value_of("algorithm").unwrap();
        let num_rays = u64::from_str(matches.value_of("num_rays").unwrap()).unwrap();
        let max_bounces = usize::from_str(matches.value_of("max_bounces").unwrap()).unwrap();

        if algorithm == "ptdr" {
            let amp = 1.0 / num_rays as f32;

            for i in 0..num_rays {
                spatializer::raytrace_ptdr(
                    &mut tracer,
                    &mut flt,
                    &world,
                    listener_pos,
                    source_pos,
                    amp,
                    &mut rng,
                    max_bounces,
                );
                if i % 16384 == 0 {
                    println!("Traced {}/{} rays...", i, num_rays);
                }
            }
        } else {
            unreachable!()
        }

        if include_direct {
            let direct_reachable = tracer.trace_finite(source_pos, listener_pos).is_none();
            if direct_reachable {
                println!("The source is visible from the listener = Yes");
            } else {
                println!("The source is visible from the listener = No");
                println!("Adding the direct path impulse anyway.");
            }

            println!("Tracing the direct path...");
            let distance = (source_pos - listener_pos).magnitude();
            flt.record_imp_dir(
                distance / world.speed_of_sound,
                FdQuant::new([1.0; 8]) / (distance * distance),
                source_pos - listener_pos,
            );
        }

        // Merge bands
        if matches.is_present("skip_bandmerge") {
            println!("Rearranging bands...");
            output.clear();
            for i in 0..num_channels {
                for k in 0..8 {
                    output.push(
                        flt.get_channel_samples(i)
                            .unwrap()
                            .iter()
                            .map(|e| e.get_ref()[k])
                            .collect(),
                    );
                }
            }
        } else {
            println!("Merging bands...");
            use ysr2::spatializer::bandmerger::{BandMerger, Lr4BandMerger};
            let mut output_buffer = vec![0.0; num_samples];

            for i in 0..num_channels {
                Lr4BandMerger::new(
                    &[
                        200.0 / sampling_rate,
                        400.0 / sampling_rate,
                        800.0 / sampling_rate,
                        1600.0 / sampling_rate,
                        3200.0 / sampling_rate,
                        6400.0 / sampling_rate,
                        12000.0 / sampling_rate,
                    ],
                ).merge(&mut output_buffer, flt.get_channel_samples(i).unwrap());

                output[i].extend(output_buffer.iter());
            }
        }

        matches
    } else {
        unreachable!();
    };

    // Normalize the output
    println!("Normalizing the output...");
    let max_gain = output
        .iter()
        .map(|channel| channel.iter().fold(0f32, |x, y| x.max(*y)))
        .fold(0f32, |x, y| x.max(y));
    println!("Maximum absolute value = {}", max_gain);
    for channel in output.iter_mut() {
        for x in channel.iter_mut() {
            *x *= 1.0 / max_gain;
        }
    }

    println!("Writing the output...");
    let spec = hound::WavSpec {
        channels: output.len() as u16,
        sample_rate: sampling_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let path = Path::new(sc_matches.value_of_os("OUTPUT").unwrap());
    let mut writer = hound::WavWriter::create(path, spec).unwrap();

    for i in 0..output[0].len() {
        for k in 0..output.len() {
            writer.write_sample(output[k][i]).unwrap();
        }
    }

    writer.finalize().unwrap();
}
