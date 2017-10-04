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
extern crate cgmath;

mod utils;
use self::utils::guspatch;

use std::path::Path;
use std::fmt;
use std::cmp;
use std::collections::BinaryHeap;
use std::cell::RefCell;
use std::marker::PhantomData;

use ysr2::common::dispatch::SerialQueue;
use ysr2::common::stream::{StreamProperties, ChannelConfig, GeneratorNode};
use ysr2::common::nodes::{Context, NodeId, NodeInputGenerator, OutputNode};
use ysr2::mixer::clipmixer::{ClipMixer, NoteId, NoteError};
use ysr2::localizer::{self, Panner};
use ysr2::localizer::nodes::{PannerNode, SourceId};
use ysr2::filters::mixer;
use ysr2::filters::reverb::{MatrixReverbNode, MatrixReverbParams};
use ysr2::filters::delay::DelayNode;

#[derive(Debug)]
struct SmfPlayer<T: Panner<NodeInputGenerator>> {
    smf: rimd::SMF,
    output_prop: StreamProperties,
    loader: guspatch::GusLoader,

    context: RefCell<Context>,
    panner_id: NodeId,
    output_id: NodeId,
    reverb_mixer_id: NodeId,

    next_events: BinaryHeap<NextEvent>,
    notes: RefCell<Vec<Note>>,
    channels: Vec<MidiChannelState>,

    /// samples per tick
    tempo: f64,
    time: f64,
    _phantom: PhantomData<T>,
}

#[derive(Debug)]
struct NextEvent {
    abs_ticks: u64,
    track: usize,
    index: usize,
}

#[derive(Debug)]
struct MidiChannelState {
    /// The current MIDI program number.
    program: u8,

    /// Panner's source ID.
    p_source_id: SourceId,

    /// Source ID used for reverb bus mixer.
    m_source_id: mixer::SourceId,

    /// Node ID of `ClipMixer` that produces audio for this channel.
    mixer_id: NodeId,
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

impl<T> SmfPlayer<T>
where
    T: Panner<NodeInputGenerator> + fmt::Debug + Send + Sync + 'static,
{
    fn new(
        smf: rimd::SMF,
        panner: T,
        output_prop: &StreamProperties,
        mut loader: guspatch::GusLoader,
    ) -> Self {
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

        // Try to preload instruments (loading during a playback would cause an
        // interruption)
        for track in smf.tracks.iter() {
            for e in track.events.iter() {
                match e.event {
                    rimd::Event::Midi(ref msg) => {
                        let ref data = msg.data;
                        let chan = data[0] & 0xf;
                        match data[0] & 0xf0 {
                            0x90 => {
                                // note on
                                let note_no = data[1];
                                if chan == 9 {
                                    loader.get_patch_for_drumset(0, note_no);
                                }
                            }
                            0xc0 => {
                                // program change
                                loader.get_patch_for_instrument(0, data[1]);
                            }
                            _ => {}
                        }
                    }
                    rimd::Event::Meta(_) => {}
                }
            }
        }

        let tempo = output_prop.sampling_rate as f64 * 0.5 / smf.division as f64;

        // Construct audio nodes
        let mut context = Context::new();
        let mut panner_node = PannerNode::new(panner, output_prop.num_channels);
        let mut reverb_mixer_node = mixer::MixerNode::new();

        // Each channel produces a monaural audio stream which is routed to
        // the panner and the reverb bus
        let channels = (0..16)
            .map(|_| {
                let mixer = ClipMixer::new(&StreamProperties {
                    num_channels: 1,
                    channel_config: ChannelConfig::Monaural,
                    ..output_prop.clone()
                });
                let mixer_id = context.insert(GeneratorNode::new(mixer, 1));
                MidiChannelState {
                    program: 0,
                    p_source_id: panner_node.insert((mixer_id, 0)),
                    m_source_id: reverb_mixer_node.insert_with_gain((mixer_id, 0), 0.3),
                    mixer_id,
                }
            })
            .collect();

        // Panner might have an intrinsic latency
        let panner_latency = panner_node.latency();

        // Insert the panner node
        let panner_id = context.insert(panner_node);

        // Insert the reverb bus mixer node
        let reverb_mixer_id = context.insert(reverb_mixer_node);

        // Insert a delay node to match the latency
        let reverb_compensated_id = if panner_latency > 0 {
            let mut delay = DelayNode::new(panner_latency);
            *delay.input_source_mut() = Some((reverb_mixer_id, 0));
            context.insert(delay)
        } else {
            reverb_mixer_id
        };

        // Insert the early reflections node
        let mut er = MatrixReverbNode::new(
            &MatrixReverbParams {
                reverb_time: 0.6 * output_prop.sampling_rate,
                mean_delay_time: 0.05 * output_prop.sampling_rate,
                diffusion: 1.0,
                reverb_time_hf_ratio: 0.9,
                high_frequency_ref: 5000.0 / output_prop.sampling_rate,
            },
            output_prop.num_channels,
        );
        *er.input_source_mut() = Some((reverb_compensated_id, 0));
        let er_id = context.insert(er);

        // Insert the late reverb node
        let late_reverb_delay_id = {
            let mut delay = DelayNode::new((0.06 * output_prop.sampling_rate) as usize);
            *delay.input_source_mut() = Some((reverb_compensated_id, 0));
            context.insert(delay)
        };
        let mut reverb = MatrixReverbNode::new(
            &MatrixReverbParams {
                reverb_time: 4.0 * output_prop.sampling_rate,
                mean_delay_time: 0.03 * output_prop.sampling_rate,
                diffusion: 1.0,
                reverb_time_hf_ratio: 0.5,
                high_frequency_ref: 5000.0 / output_prop.sampling_rate,
            },
            output_prop.num_channels,
        );
        *reverb.input_source_mut() = Some((late_reverb_delay_id, 0));
        let reverb_id = context.insert(reverb);

        // Mix the output of the panner and the reverb
        let final_mixer_outputs: Vec<_> = (0..output_prop.num_channels)
            .map(|i| {
                let mut final_mixer = mixer::MixerNode::new();
                final_mixer.insert((panner_id, i));

                static ER_GAIN: f64 = 4.0;
                static LATE_GAIN: f64 = 2.0;
                static XF_AMOUNT: f64 = 0.2;
                final_mixer.insert_with_gain((reverb_id, i), LATE_GAIN * (1.0 - XF_AMOUNT));
                final_mixer.insert_with_gain((er_id, i), ER_GAIN * (1.0 - XF_AMOUNT));

                final_mixer.insert_with_gain((reverb_id, 1 - i), LATE_GAIN * XF_AMOUNT);
                final_mixer.insert_with_gain((er_id, 1 - i), ER_GAIN * XF_AMOUNT);

                let final_mixer_id = context.insert(final_mixer);
                (final_mixer_id, 0)
            })
            .collect();

        // Insert the output node
        let mut output = OutputNode::new(output_prop.num_channels);
        for (i, fm_output) in final_mixer_outputs.iter().enumerate() {
            *output.input_source_mut(i).unwrap() = Some(*fm_output);
        }
        let output_id = context.insert(output);

        Self {
            smf,
            loader,
            output_prop: output_prop.clone(),

            context: RefCell::new(context),
            panner_id,
            output_id,
            reverb_mixer_id,

            notes: RefCell::new(Vec::new()),
            channels,
            next_events,

            tempo,
            time: 0.0,

            _phantom: PhantomData,
        }
    }

    fn note_off(&self, delay: u64, note_no: u8, chan: u8) {
        self.notes.borrow_mut().retain(
            |note| if note.note == note_no &&
                note.channel == chan
            {
                let mut context = self.context.borrow_mut();
                let mixer = context
                    .get_mut_as::<GeneratorNode<ClipMixer>>(&self.channels[chan as usize].mixer_id)
                    .unwrap()
                    .get_ref_mut();
                match (|| {
                    mixer.set_gain(
                        delay,
                        note.note_id,
                        0.0,
                        self.output_prop.sampling_rate * 0.25,
                    )?;
                    mixer.stop(
                        delay + (self.output_prop.sampling_rate * 0.5) as u64,
                        note.note_id,
                    )?;
                    Ok(())
                })() {
                    Ok(()) => {}

                    // Some instruments doesn't have loop
                    Err(NoteError::UnknownId) => {}
                }
                false
            } else {
                true
            },
        );
    }
}

trait Player {
    fn is_done(&self) -> bool;
    fn render(&mut self, buffer: &mut [f32]);
}

impl<T> Player for SmfPlayer<T>
where
    T: Panner<NodeInputGenerator> + fmt::Debug + Send + Sync + 'static,
{
    fn is_done(&self) -> bool {
        if self.next_events.is_empty() && self.notes.borrow().is_empty() {
            use ysr2::common::stream::Generator;
            let context = self.context.borrow();
            for channel in self.channels.iter() {
                if context
                    .get_as::<GeneratorNode<ClipMixer>>(&channel.mixer_id)
                    .unwrap()
                    .get_ref()
                    .is_active()
                {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    fn render(&mut self, buffer: &mut [f32]) {
        let num_samples = buffer.len() / 2;

        // process MIDI events
        let mut sample_offs = 0f64;
        loop {
            let new_next_event = if let Some(next_event) = self.next_events.peek() {
                let evt_sample_offs = (next_event.abs_ticks as f64 - self.time) * self.tempo +
                    sample_offs;
                let evt_sample_offs_i = evt_sample_offs as u64;
                if evt_sample_offs_i >= num_samples as u64 {
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
                                } else if chan != 9 {
                                    // melodic channel
                                    let pitch = 440.0 * ((note_no as f64 - 69.0) / 12.0).exp2();

                                    let ref channel = self.channels[chan as usize];
                                    let mixer_id = channel.mixer_id;

                                    let patch =
                                        self.loader.get_patch_for_instrument(0, channel.program);
                                    let waveform = patch.choose_waveform(pitch);

                                    let note_id = self.context
                                        .borrow_mut()
                                        .get_mut_as::<GeneratorNode<ClipMixer>>(&mixer_id)
                                        .unwrap()
                                        .get_ref_mut()
                                        .build_note(&waveform.clip)
                                        .pitch(pitch)
                                        .gain(vel * vel * 0.05)
                                        .start(evt_sample_offs_i);

                                    self.notes.borrow_mut().push(Note {
                                        channel: chan,
                                        note: note_no,
                                        note_id,
                                    });
                                } else {
                                    // rhythm channel
                                    let ref channel = self.channels[chan as usize];
                                    let mixer_id = channel.mixer_id;

                                    if let Some(patch) = self.loader.get_patch_for_drumset(
                                        channel.program,
                                        note_no,
                                    )
                                    {
                                        let ref waveform = patch.waveforms[0];
                                        let note_id = self.context
                                            .borrow_mut()
                                            .get_mut_as::<GeneratorNode<ClipMixer>>(&mixer_id)
                                            .unwrap()
                                            .get_ref_mut()
                                            .build_note(&waveform.clip)
                                            .pitch(waveform.root_freq)
                                            .gain(vel * vel * 0.05)
                                            .start(evt_sample_offs_i);

                                        self.notes.borrow_mut().push(Note {
                                            channel: chan,
                                            note: note_no,
                                            note_id,
                                        });
                                    }
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
                            0xb0 => {
                                // control change
                                let control = data[1];
                                let value = data[2];
                                match control {
                                    0x0a => {
                                        // Pan
                                        let azimuth = (value as f64 / 128.0 - 0.5) *
                                            std::f64::consts::PI;
                                        let dir =
                                            cgmath::Vector3::new(azimuth.sin(), 0.0, azimuth.cos());
                                        let p_source_id = self.channels[chan as usize].p_source_id;
                                        self.context
                                            .borrow_mut()
                                            .get_mut_as::<PannerNode<T>>(&self.panner_id)
                                            .unwrap()
                                            .direction_mut(&p_source_id)
                                            .unwrap()
                                            .set_slow(dir, self.output_prop.sampling_rate * 0.01);
                                    }
                                    0x5b => {
                                        // Effect 1 (reverb send)
                                        let gain = value as f64 / 127.0;
                                        let m_source_id = self.channels[chan as usize].m_source_id;
                                        self.context
                                            .borrow_mut()
                                            .get_mut_as::<mixer::MixerNode>(&self.reverb_mixer_id)
                                            .unwrap()
                                            .gain_mut(&m_source_id)
                                            .unwrap()
                                            .set_slow(gain, self.output_prop.sampling_rate * 0.01);
                                    }
                                    _ => {}
                                }
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

        self.time += (num_samples as f64 - sample_offs) / self.tempo;

        // Request a next frame
        let mut context = self.context.borrow_mut();
        {
            let output = context.get_mut_as::<OutputNode>(&self.output_id).unwrap();
            output.request_frame(num_samples);
        }

        // Process the frame
        context.render().unwrap();

        // Read the output
        {
            let output = context.get_as::<OutputNode>(&self.output_id).unwrap();
            let buf1 = output.get_samples(0).unwrap();
            let buf2 = output.get_samples(1).unwrap();

            // Convert to the interleaved stereo format
            for i in 0..num_samples {
                buffer[i * 2] = buf1[i];
                buffer[i * 2 + 1] = buf2[i];
            }
        }
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
        .arg(
            Arg::with_name("gusconfig")
                .short("c")
                .long("cfg")
                .value_name("GUSCFG")
                .help(
                    "Specifies the path to a GUS (Gravis Ultrasound) configuration file",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("panner")
                .short("p")
                .long("panner")
                .value_name("PANNER")
                .help("Specifies the panner")
                .takes_value(true)
                .possible_values(&["equalpower", "hrtf"])
                .default_value("equalpower"),
        )
        .arg(
            Arg::with_name("benchmark")
                .short("b")
                .long("benchmark")
                .help(
                    "Renders the output as fast as possible. The audio output \
                       will be disabled in this mode.",
                ),
        )
        .get_matches();

    // Load the input SMF file
    println!("Loading the input file");
    let input_path = matches.value_of("INPUT").unwrap();
    let smf = rimd::SMF::from_file(Path::new(input_path)).unwrap();

    // Load the GUS patch data
    let gus_loader = if let Some(cfg_path) = matches.value_of_os("gusconfig") {
        println!("Loading the GUS config file");
        guspatch::GusLoader::from_config_path(&Path::new(cfg_path)).unwrap()
    } else {
        guspatch::GusLoader::default()
    };

    // Initialize the player
    println!("Initializing the player");
    let prop = StreamProperties {
        sampling_rate: 44100f64,
        num_channels: 2,
        channel_config: ChannelConfig::Stereo,
    };

    let panner_name = matches.value_of("panner").unwrap();
    let mut player: Box<Player> = if panner_name == "equalpower" {
        let panner = localizer::equalpower::EqualPowerPanner::new(SerialQueue);
        Box::new(SmfPlayer::new(smf, panner, &prop, gus_loader))
    } else if panner_name == "hrtf" {
        let panner = localizer::hrtf::HrtfPanner::new(SerialQueue);
        Box::new(SmfPlayer::new(smf, panner, &prop, gus_loader))
    } else {
        unreachable!()
    };

    if matches.is_present("benchmark") {
        // Benchmark mode - render as fast as possible
        let mut buffer = vec![0.0; 16384];
        while !player.is_done() {
            player.render(&mut buffer);
        }
        return;
    }

    // Initialize PortAudio
    println!("Initializing the audio device");
    let pa = portaudio::PortAudio::new().unwrap();

    // Specify a large frame size because on macOS it defaults to an unbelievably
    // small value which sometimes causes the playback to stutter a lot,
    // especially on the debug build.
    let settings = pa.default_output_stream_settings(2, prop.sampling_rate, 4096)
        .unwrap();
    let mut stream = pa.open_non_blocking_stream(settings, move |mut output| {
        let ref mut args: portaudio::stream::OutputCallbackArgs<f32> = output;
        if player.is_done() {
            std::process::exit(0);
        }
        player.render(args.buffer);
        portaudio::stream::CallbackResult::Continue
    }).unwrap();

    println!("Starting...");
    stream.start().unwrap();

    let mut line_input = String::new();

    println!("Hit return to terminate the playback");
    std::io::stdin().read_line(&mut line_input).unwrap();
}
