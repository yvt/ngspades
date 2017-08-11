//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate yfft;
extern crate byteorder;

use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
use std::fs::File;
use std::env;
use std::str::FromStr;
use std::collections::BTreeMap;

fn main() {
    let mut ir_fft_hc_builder = IrFftHcBuilder::new();

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("data.rs");
    let mut f = File::create(&dest_path).unwrap();

    write!(f, "static KEMAR_DATA_INTERNAL: [Ring; 14] = [\n").unwrap();

    let mut elev = -40i32;
    while elev <= 90 {
        write!(f, "    Ring {{\n").unwrap();
        write!(f, "        elevation: {},\n", elev).unwrap();
        write!(f, "        samples: &[\n").unwrap();

        let elev_dir = Path::new(&format!("elev{}", elev)).read_dir().expect(
            "read_dir failed",
        );
        let elev_pfx = format!("H{}e", elev);
        let elev_sfx = "a.wav";
        let mut ents = BTreeMap::new();

        for entry in elev_dir {
            if let Ok(entry) = entry {
                let fname = entry.file_name();
                let fname = fname.to_string_lossy();
                if !fname.starts_with(&elev_pfx) || !fname.ends_with(&elev_sfx) {
                    continue;
                }

                let azimuth = i32::from_str(&fname[elev_pfx.len()..fname.len() - elev_sfx.len()])
                    .expect("failed to parse azimuth number");
                assert!(azimuth >= 0, "azimuth must be >= 0: {}", fname);
                assert!(azimuth <= 180, "azimuth must be <= 180: {}", fname);

                ents.insert(azimuth, entry.path());
            }
        }

        assert!(
            ents.contains_key(&0),
            "data for azimuth 0 not found at \
            elevation {}",
            elev
        );

        for (&azimuth, path) in ents.iter() {
            let wave = read_wave(path);
            let ir_fft_hcs = [
                ir_fft_hc_builder.process(&wave[0]),
                ir_fft_hc_builder.process(&wave[1]),
            ];

            write!(f, "            Sample {{\n").unwrap();
            write!(f, "                azimuth: {},\n", azimuth).unwrap();
            write!(f, "                _pad: [0, 0, 0],\n").unwrap();
            write!(f, "                ir_fft_hc: [\n").unwrap();

            for ir_fft_hc in ir_fft_hcs.iter() {
                write!(f, "                    [").unwrap();
                for point in ir_fft_hc.iter() {
                    write!(f, "{}f32, ", point).unwrap();
                }
                write!(f, "],\n").unwrap();
            }
            write!(f, "                ],\n").unwrap();
            write!(f, "            }},\n").unwrap();
        }

        write!(f, "        ],\n").unwrap();
        write!(f, "    }},\n").unwrap();

        elev += 10;
    }


    write!(f, "];\n").unwrap();
}

fn read_wave(path: &Path) -> [[f32; 128]; 2] {
    use std::io::Cursor;
    use byteorder::{LittleEndian, ReadBytesExt};

    let mut f = File::open(path).unwrap();
    f.seek(SeekFrom::Start(44)).unwrap();

    let mut buffer = [0u8; 128 * 2 * 2];
    f.read_exact(&mut buffer).unwrap();

    let mut reader = Cursor::new(&buffer[..]);

    let mut samples = [[0f32; 128]; 2];
    for i in 0..128 {
        for ch in 0..2 {
            samples[ch][i] = reader.read_i16::<LittleEndian>().unwrap() as f32 / 32768f32;
        }
    }

    samples
}

struct IrFftHcBuilder {
    env: yfft::Env<f32, yfft::Setup<f32>>,
}

impl IrFftHcBuilder {
    fn new() -> Self {
        use yfft::{DataOrder, DataFormat};
        let setup = yfft::Setup::new(&yfft::Options {
            input_data_order: DataOrder::Natural,
            output_data_order: DataOrder::Natural,
            input_data_format: DataFormat::Real,
            output_data_format: DataFormat::HalfComplex,
            len: 256,
            inverse: false,
        }).unwrap();
        Self { env: yfft::Env::new(setup) }
    }

    fn process(&mut self, samples: &[f32]) -> [f32; 256] {
        let mut data = [0f32; 256];
        data[0..128].copy_from_slice(&samples);
        self.env.transform(&mut data);
        data
    }
}
