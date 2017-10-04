//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Limited GUS (Gravis Ultrasound) patch file reader.
extern crate ysr2;
extern crate byteorder;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Result, Error, ErrorKind};
use self::ysr2::common::stream::{StreamProperties, ChannelConfig};
use self::ysr2::mixer::clip::Clip;
use self::byteorder::{LE, ReadBytesExt};

#[derive(Debug, Clone)]
pub struct GusWaveform {
    pub name: String,
    /// `Clip` that must be played with the pitch value equal to the frequency of
    /// the node.
    pub clip: Clip,

    /// The minimum frequency of the node for which this waveform is applicable.
    pub min_freq: f64,

    /// The maximum frequency of the node for which this waveform is applicable.
    pub max_freq: f64,
}

#[derive(Debug, Clone)]
pub struct GusPatch {
    description: String,
    waveforms: Vec<GusWaveform>,
}

fn nul_terminate(s: &str) -> &str {
    if let Some(i) = s.find("\0") {
        &s[0..i]
    } else {
        s
    }
}

impl GusPatch {
    pub fn from_reader<T: Read + Seek>(f: &mut T) -> Result<Self> {
        let mut magic = [0; 22];
        f.read_exact(&mut magic)?;
        if "GF1PATCH110\0ID#000002\0".as_bytes() != magic &&
            "GF1PATCH100\0ID#000002\0".as_bytes() != magic
        {
            return Err(Error::new(ErrorKind::Other, "invalid magic"));
        }

        let mut description = [0; 60];
        f.read_exact(&mut description)?;
        let description = nul_terminate(&String::from_utf8_lossy(&description)).to_owned();

        let num_instruments = f.read_u8()?;
        if num_instruments != 0 && num_instruments != 1 {
            return Err(Error::new(
                ErrorKind::Other,
                "unsupported value of # of instruments",
            ));
        }

        // # of voices: doesn't make sense!
        f.read_u8()?;

        let num_channels = f.read_u8()?;
        if num_channels != 1 && num_channels != 0 {
            return Err(Error::new(
                ErrorKind::Other,
                "unsupported value of # of channels",
            ));
        }

        // just skip the unidentified part of the header...
        // (some says that there is a list of instruments here, but for now we
        // assume exactly one instrument)
        f.seek(SeekFrom::Current(198 - 85))?;

        let num_waves = f.read_u8()?;

        // just skip the unidentified part of the header...
        f.seek(SeekFrom::Current(239 - 199))?;

        let mut waveforms = Vec::new();

        for _ in 0..num_waves {
            let mut wave_name = [0; 7];
            f.read_exact(&mut wave_name)?;
            let wave_name = nul_terminate(&String::from_utf8_lossy(&wave_name)).to_owned();

            // Loop offset fractions: we can't do anything about that
            // (Seriously, how are we supposed to interpolate the waveform with
            // fractional loop end points)
            let _fractions = f.read_u8()?;

            let sample_data_size = f.read_u32::<LE>()?;
            let mut loop_start = f.read_u32::<LE>()?;
            let mut loop_end = f.read_u32::<LE>()?;
            let sample_rate = f.read_u16::<LE>()?;
            let low_freq = f.read_u32::<LE>()?;
            let high_freq = f.read_u32::<LE>()?;
            let root_freq = f.read_u32::<LE>()?;
            f.read_u16::<LE>()?; // tuning (wtf?)
            f.read_u8()?; // panning

            // Attack, decay, sustain, and release x 3
            let mut env_rates = [0; 6];
            let mut env_offsets = [0; 6];
            f.read_exact(&mut env_rates)?;
            f.read_exact(&mut env_offsets)?;

            let _tremolo_sweep = f.read_u8()?;
            let _tremolo_rate = f.read_u8()?;
            let _tremolo_depth = f.read_u8()?;
            let _vibrato_sweep = f.read_u8()?;
            let _vibrato_rate = f.read_u8()?;
            let _vibrato_depth = f.read_u8()?;
            let sampling_mode = f.read_u8()?;
            let is_16bit = (sampling_mode & 1) != 0;
            let is_unsigned = (sampling_mode & 2) != 0;
            let is_looping = (sampling_mode & 4) != 0;

            let _scale_freq = f.read_u16::<LE>()?;
            let _scale_factor = f.read_u16::<LE>()?;

            // Reserved
            f.seek(SeekFrom::Current(36))?;

            let num_samples = if is_16bit {
                loop_start /= 2;
                loop_end /= 2;
                sample_data_size / 2
            } else {
                sample_data_size
            } as usize;

            let prop = StreamProperties {
                sampling_rate: sample_rate as f64 / (root_freq as f64 / 1000.0),
                num_channels: 1,
                channel_config: ChannelConfig::Monaural,
            };
            let loop_start = if is_looping {
                Some(loop_start as usize)
            } else {
                None
            };
            let loaded_num_samples = if is_looping {
                if loop_end as usize > num_samples {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!(
                            "loop end point extends beyond the boundary for the instrument '{}'",
                            &wave_name
                        ),
                    ));
                }
                loop_end as usize
            } else {
                num_samples
            };

            let clip = Clip::new(loaded_num_samples, loop_start, &prop);
            {
                let mut writer = clip.write_samples();
                let chan = writer.get_channel_mut(0);

                if is_16bit {
                    if is_unsigned {
                        for i in 0..loaded_num_samples {
                            chan[i] = f.read_u16::<LE>()? as f32 * (2.0 / 65536.0) - 1.0;
                        }
                    } else {
                        for i in 0..loaded_num_samples {
                            chan[i] = f.read_i16::<LE>()? as f32 * (1.0 / 32767.0);
                        }
                    }
                } else {
                    if is_unsigned {
                        for i in 0..loaded_num_samples {
                            chan[i] = f.read_u8()? as f32 * (2.0 / 256.0) - 1.0;
                        }
                    } else {
                        for i in 0..loaded_num_samples {
                            chan[i] = f.read_i8()? as f32 * (1.0 / 127.0);
                        }
                    }
                }
            }

            let remaining_samples = num_samples - loaded_num_samples;
            if is_16bit {
                f.seek(SeekFrom::Current(remaining_samples as i64 * 2))?;
            } else {
                f.seek(SeekFrom::Current(remaining_samples as i64))?;
            }

            waveforms.push(GusWaveform {
                name: wave_name,
                clip,
                min_freq: low_freq as f64 / 1000.0,
                max_freq: high_freq as f64 / 1000.0,
            });
        }

        Ok(GusPatch {
            description,
            waveforms,
        })
    }

    pub fn choose_waveform(&self, note_freq: f64) -> &GusWaveform {
        self.waveforms
            .iter()
            .map(|waveform| {
                let dist = if note_freq < waveform.min_freq {
                    waveform.min_freq - note_freq
                } else if note_freq > waveform.max_freq {
                    note_freq - waveform.max_freq
                } else {
                    0.0
                };
                (waveform, dist)
            })
            .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap())
            .unwrap()
            .0
    }
}

#[derive(Debug, Clone)]
pub struct GusConfig {
    program_map: HashMap<(u8, u8), String>,
    drumset_map: HashMap<(u8, u8), String>,
}

impl GusConfig {
    pub fn from_string(s: &str) -> Result<Self> {
        enum State {
            Program(u8),
            Drumset(u8),
        }
        let mut state = State::Program(0);

        let newline = match (s.find("\r"), s.find("\n")) {
            (Some(_), Some(_)) => "\r\n",
            (Some(_), None) => "\r",
            _ => "\n",
        };

        let mut program_map = HashMap::new();
        let mut drumset_map = HashMap::new();

        for line in s.split(newline) {
            let line = if let Some(comment_start) = line.find("#") {
                &line[0..comment_start]
            } else {
                line
            }.trim();
            if line.len() == 0 {
                continue;
            }
            let mut parts = line.split_whitespace();
            let first = parts.next().unwrap();
            if first == "bank" {
                let id = parts.next().ok_or(Error::new(
                    ErrorKind::Other,
                    "bank's parameter missing",
                ))?;
                let id = <u8>::from_str_radix(id, 10).or(Err(Error::new(
                    ErrorKind::Other,
                    "bank's parameter is invalid",
                )))?;
                state = State::Program(id);
            } else if first == "drumset" {
                let id = parts.next().ok_or(Error::new(
                    ErrorKind::Other,
                    "drumset's parameter missing",
                ))?;
                let id = <u8>::from_str_radix(id, 10).or(Err(Error::new(
                    ErrorKind::Other,
                    "drumset's parameter is invalid",
                )))?;
                state = State::Drumset(id);
            } else {
                let num = <u8>::from_str_radix(first, 10).or(Err(Error::new(
                    ErrorKind::Other,
                    format!(
                        "instrument number is invalid: {}",
                        first
                    ),
                )))?;
                let name = parts.next().ok_or(Error::new(
                    ErrorKind::Other,
                    "intrument name missing",
                ))?;
                match state {
                    State::Program(bank) => {
                        program_map.insert((bank, num), name.into());
                    }
                    State::Drumset(program) => {
                        drumset_map.insert((program, num), name.into());
                    }
                }
            }
        }

        Ok(Self {
            program_map,
            drumset_map,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GusLoader {
    patches: HashMap<String, GusPatch>,
    cfg: GusConfig,
    base_path: PathBuf,
}

impl GusLoader {
    pub fn from_config_path(cfg_path: &Path) -> Result<Self> {
        let cfg = {
            let mut file = File::open(cfg_path)?;
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;
            GusConfig::from_string(&buffer)?
        };
        let base_path = cfg_path
            .parent()
            .ok_or(Error::new(
                ErrorKind::Other,
                "cannot determine the base path",
            ))?
            .to_path_buf();
        Ok(GusLoader {
            patches: HashMap::new(),
            cfg,
            base_path,
        })
    }

    /// Create a `GusLoader` with an internally generated note sound.
    pub fn default() -> Self {
        let waveform = GusWaveform {
            name: "default".to_owned(),
            clip: make_wave(),
            min_freq: 1.0,
            max_freq: 99999.0,
        };
        let patch = GusPatch {
            description: "default".to_owned(),
            waveforms: vec![waveform],
        };
        GusLoader {
            patches: [("default".to_owned(), patch)].iter().cloned().collect(),
            cfg: GusConfig {
                program_map: [((0, 0), "default".to_owned())].iter().cloned().collect(),
                drumset_map: HashMap::new(),
            },
            base_path: PathBuf::new(),
        }
    }

    pub fn get_patch(&mut self, name: &str) -> &GusPatch {
        let lower = name.to_lowercase();
        let base_path = &self.base_path;
        self.patches.entry(lower).or_insert_with(|| {
            use std::io::BufReader;
            let rel_path = Path::new(name);
            let path = base_path.join(rel_path);
            println!("GusLoader: Loading {:?}", path);
            let file = File::open(&path)
                .or_else(|_| File::open(path.with_extension("pat")))
                .unwrap();
            let patch = GusPatch::from_reader(&mut BufReader::new(file)).unwrap();
            patch
        })
    }

    pub fn get_patch_for_instrument(&mut self, bank: u8, program: u8) -> &GusPatch {
        let name = self.cfg
            .program_map
            .get(&(bank, program))
            .or_else(|| self.cfg.program_map.get(&(0, program)))
            .or_else(|| self.cfg.program_map.get(&(0, 0)))
            .ok_or(Error::new(
                ErrorKind::Other,
                "cannot determine the default patch",
            ))
            .unwrap()
            .clone();
        self.get_patch(&name)
    }

    pub fn get_patch_for_drumset(&mut self, program: u8, note: u8) -> Option<&GusPatch> {
        let name = self.cfg
            .drumset_map
            .get(&(program, note))
            .or_else(|| self.cfg.drumset_map.get(&(0, note)))
            .map(Clone::clone);
        if let Some(name) = name {
            Some(self.get_patch(&name))
        } else {
            None
        }
    }
}

fn make_wave() -> Clip {
    let period: usize = 64;
    let cycles: usize = 4096;
    let prop = StreamProperties {
        sampling_rate: period as f64,
        num_channels: 1,
        channel_config: ChannelConfig::Monaural,
    };
    let clip = Clip::new(period * cycles, Some(period * (cycles - 1)), &prop);
    {
        let mut writer = clip.write_samples();
        let chan = writer.get_channel_mut(0);
        let rho = ::std::f32::consts::PI * 2.0 / period as f32;
        for i in 0..chan.len() {
            let mut s = 0f32;
            let damp_time = if i < period * (cycles - 1) {
                i as f32
            } else {
                // looping part
                period as f32 * (cycles - 1) as f32
            };
            for k in 1..32 {
                // higher order harmonics decay faster than lower ones
                let mut gain = (-damp_time * (k as f32 * 0.5 + 1.0) * 0.0001).exp2();

                // colorize our tone in arbitrary way
                if k > 1 {
                    gain *= (k as f32 * (1.0 + damp_time / period as f32 * 0.005)).cos();
                    gain *= (k as f32 * -0.05).exp2();
                    gain *= 1.0 - k as f32 / 32.0;
                }

                s += (k as f32 * i as f32 * rho).cos() * gain;
            }

            chan[i] = s;
        }
    }
    clip
}
