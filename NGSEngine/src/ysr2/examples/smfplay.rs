//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Standard MIDI Format (SMF) file player made using YSR2.
//!
//! It is recommended that you use the release build option to run this example
//! due to the real-time
extern crate ysr2;
extern crate rimd;
extern crate clap;
extern crate portaudio;

use std::path::Path;
use std::cmp;
use std::collections::BinaryHeap;
use std::cell::RefCell;

use ysr2::common::stream::{StreamProperties, ChannelConfig, Generator};
use ysr2::mixer::clip::Clip;
use ysr2::mixer::clipmixer::{ClipMixer, NoteId};

fn make_wave() -> Clip {
    let period: usize = 64;
    let cycles: usize = 4096;
    let prop = StreamProperties {
        sampling_rate: 440.0 * (period as f64),
        num_channels: 1,
        channel_config: ChannelConfig::Monaural,
    };
    let clip = Clip::new(period * cycles, Some(period * (cycles - 1)), &prop);
    {
        let mut writer = clip.write_samples();
        let mut chan = writer.get_channel_mut(0);
        let rho = std::f32::consts::PI * 2.0 / period as f32;
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

#[derive(Debug)]
struct SmfPlayer {
    smf: rimd::SMF,
    clip: Clip,
    next_events: BinaryHeap<NextEvent>,
    mixer: RefCell<ClipMixer>,
    output_prop: StreamProperties,
    notes: RefCell<Vec<Note>>,
    channels: [MidiChannelState; 16],
    /// samples per tick
    tempo: f64,
    time: f64,
}

#[derive(Debug)]
struct NextEvent {
    abs_ticks: u64,
    track: usize,
    index: usize,
}

#[derive(Debug)]
struct MidiChannelState {
    program: u8,
}

#[derive(Debug)]
struct Note {
    channel: u8,
    note: u8,
    note_id: NoteId,
}

impl Ord for NextEvent {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.abs_ticks.cmp(&other.abs_ticks).reverse()
    }
}
impl PartialOrd for NextEvent {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for NextEvent {
    fn eq(&self, other: &Self) -> bool {
        self.abs_ticks == other.abs_ticks
    }
}
impl Eq for NextEvent {}

impl Default for MidiChannelState {
    fn default() -> Self {
        MidiChannelState { program: 0 }
    }
}

impl SmfPlayer {
    fn new(smf: rimd::SMF, output_prop: &StreamProperties) -> Self {
        let mut next_events = BinaryHeap::new();
        for (i, track) in smf.tracks.iter().enumerate() {
            if let Some(e) = track.events.iter().nth(0) {
                next_events.push(NextEvent {
                    abs_ticks: e.vtime,
                    track: i,
                    index: 0,
                });
            }
        }

        let tempo = output_prop.sampling_rate as f64 * 0.5 / smf.division as f64;

        Self {
            smf,
            clip: make_wave(),
            next_events,
            mixer: RefCell::new(ClipMixer::new(output_prop)),
            output_prop: output_prop.clone(),
            notes: RefCell::new(Vec::new()),
            channels: Default::default(),
            tempo,
            time: 0.0,
        }
    }

    fn is_done(&self) -> bool {
        self.next_events.is_empty() && self.notes.borrow().is_empty() &&
            !self.mixer.borrow().is_active()
    }

    fn note_off(&self, delay: u64, note_no: u8, chan: u8) {
        self.notes.borrow_mut().retain(
            |note| if note.note == note_no &&
                note.channel == chan
            {
                let mut mixer = self.mixer.borrow_mut();
                mixer
                    .set_gain(
                        delay,
                        note.note_id,
                        0.0,
                        self.output_prop.sampling_rate * 0.5,
                    )
                    .unwrap();
                mixer
                    .stop(
                        delay + (self.output_prop.sampling_rate * 0.5) as u64,
                        note.note_id,
                    )
                    .unwrap();
                false
            } else {
                true
            },
        );
    }

    fn render(&mut self, buffer: &mut [f32]) {
        // process MIDI events
        let mut sample_offs = 0f64;
        loop {
            let new_next_event = if let Some(next_event) = self.next_events.peek() {
                let evt_sample_offs = (next_event.abs_ticks as f64 - self.time) * self.tempo +
                    sample_offs;
                let evt_sample_offs_i = evt_sample_offs as u64;
                if evt_sample_offs_i >= buffer.len() as u64 {
                    break;
                }

                self.time = next_event.abs_ticks as f64;
                sample_offs = evt_sample_offs;

                let ref track = self.smf.tracks[next_event.track];
                let ref event: rimd::Event = track.events[next_event.index].event;

                match event {
                    &rimd::Event::Midi(ref msg) => {
                        let ref data = msg.data;
                        let chan = data[0] & 0xf;
                        match data[0] & 0xf0 {
                            0x90 => {
                                // note on
                                let note_no = data[1];
                                let vel = (data[2] as f64) / 127.0;
                                if vel == 0.0 {
                                    // 0 velocity note on is note off
                                    self.note_off(evt_sample_offs_i, note_no, chan);
                                } else if chan != 9 && self.channels[chan as usize].program < 120 {
                                    // ignore the rhythm channel and SFXs
                                    let pitch = ((note_no as f64 - 69.0) / 12.0).exp2();

                                    let note_id = self.mixer
                                        .borrow_mut()
                                        .build_note(&self.clip)
                                        .pitch(pitch)
                                        .gain(vel * vel * 0.05)
                                        .start(evt_sample_offs_i);

                                    self.notes.borrow_mut().push(Note {
                                        channel: chan,
                                        note: note_no,
                                        note_id,
                                    });
                                }
                            }
                            0x80 => {
                                // note off
                                let note_no = data[1];
                                self.note_off(evt_sample_offs_i, note_no, chan);
                            }
                            0xc0 => {
                                // program change
                                self.channels[chan as usize].program = data[1];
                            }

                            _ => {}
                        }
                    }
                    &rimd::Event::Meta(ref msg) => {
                        match msg {
                            &rimd::MetaEvent {
                                command: rimd::MetaCommand::TempoSetting,
                                ref data,
                                ..
                            } => {
                                // microseconds per beat
                                let tempo = data[2] as u32 | ((data[1] as u32) << 8) |
                                    ((data[0] as u32) << 16);
                                self.tempo = tempo as f64 / 1000000.0 / self.smf.division as f64 *
                                    self.output_prop.sampling_rate;
                            }
                            _ => {}
                        }
                    }
                }

                // Check the next event
                if next_event.index + 1 == track.events.len() {
                    None
                } else {
                    Some(NextEvent {
                        abs_ticks: next_event.abs_ticks + track.events[next_event.index + 1].vtime,
                        track: next_event.track,
                        index: next_event.index + 1,
                    })
                }
            } else {
                break;
            };

            self.next_events.pop();
            if let Some(new_next_event) = new_next_event {
                self.next_events.push(new_next_event);
            }
        }

        self.time += (buffer.len() as f64 - sample_offs) / self.tempo;

        let range = 0..buffer.len();
        self.mixer.borrow_mut().render(&mut [buffer], range);
    }
}

fn main() {
    use clap::{App, Arg};
    // Use `clap` to parse command-line arguments
    let matches = App::new("smfplay")
        .author("yvt <i@yvt.jp>")
        .about("plays a SMF (standard MIDI format) file using YSR2")
        .arg(
            Arg::with_name("INPUT")
                .help("SMF file to play")
                .required(true)
                .index(1),
        )
        .get_matches();

    // Load the input SMF file
    let input_path = matches.value_of("INPUT").unwrap();
    let smf = rimd::SMF::from_file(Path::new(input_path)).unwrap();
    let prop = StreamProperties {
        sampling_rate: 44100f64,
        num_channels: 1,
        channel_config: ChannelConfig::Monaural,
    };
    let mut smf_player = SmfPlayer::new(smf, &prop);

    // Initialize PortAudio
    let pa = portaudio::PortAudio::new().unwrap();

    // Specify a large frame size because on macOS it defaults to an unbelievably
    // small value which sometimes causes the playback to stutter a lot,
    // especially on the debug build.
    let settings = pa.default_output_stream_settings(1, prop.sampling_rate, 4096)
        .unwrap();
    let mut stream = pa.open_non_blocking_stream(settings, move |mut output| {
        let ref mut args: portaudio::stream::OutputCallbackArgs<f32> = output;
        if smf_player.is_done() {
            std::process::exit(0);
        }
        smf_player.render(args.buffer);
        portaudio::stream::CallbackResult::Continue
    }).unwrap();

    stream.start().unwrap();

    let mut line_input = String::new();

    println!("Hit return to terminate the playback");
    std::io::stdin().read_line(&mut line_input).unwrap();
}
