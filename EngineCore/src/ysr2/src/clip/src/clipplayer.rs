//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Produces an audio stream using a given `Clip`, with a simple sampling rate
//! conversion.
use std::collections::BinaryHeap;
use std::cmp;
use std::ops::Range;

use ysr2_common::stream::StreamProperties;
use ysr2_common::values::DynamicValue;
use ysr2_common::slicezip::{SliceZipMut, IndexByVal, IndexByValMut};
use clip::{Clip, WAVE_PAD_LEN};
use event::Event;

#[derive(Debug)]
pub struct ClipPlayer {
    clip: Clip,
    output_prop: StreamProperties,

    /// Padded sample position in the source clip.
    ///
    /// Starts from `WAVE_PAD_LEN as f64 - 3.0`.
    position: f64,
    pitch: DynamicValue,
    gain: DynamicValue,

    events: BinaryHeap<EventAssoc>,
    cur_iter: u32,
}

/// Event to be signalled by an `ClipPlayer`.
///
/// `event` will be set when `ClipPlayer::position` becomes equal to or greater
/// than `position`. `ClipPlayer::cur_iter` must match `EventAssoc::iter`.
#[derive(Debug)]
struct EventAssoc {
    iter: u32,
    position: f64,
    event: Event,
}

impl Ord for EventAssoc {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.iter
            .cmp(&other.iter)
            .then(self.position.partial_cmp(&other.position).unwrap())
            .reverse() // `BinaryHeap` is max-heap
    }
}
impl PartialOrd for EventAssoc {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for EventAssoc {
    fn eq(&self, other: &Self) -> bool {
        (self.iter, self.position) == (other.iter, other.position)
    }
}
impl Eq for EventAssoc {}

impl ClipPlayer {
    pub fn new(clip: &Clip, output_prop: &StreamProperties) -> Self {
        assert_eq!(
            clip.stream_properties().num_channels,
            output_prop.num_channels
        );
        assert_eq!(
            clip.stream_properties().channel_config,
            output_prop.channel_config
        );

        ClipPlayer {
            clip: clip.clone(),
            output_prop: output_prop.clone(),
            position: WAVE_PAD_LEN as f64 - 3.0,
            pitch: DynamicValue::new(1.0),
            gain: DynamicValue::new(1.0),
            events: BinaryHeap::new(),
            cur_iter: 0,
        }
    }

    pub fn output_properties(&self) -> &StreamProperties {
        &self.output_prop
    }

    pub fn clip(&self) -> &Clip {
        &self.clip
    }

    /// Retrieve the current playback position.
    ///
    /// FIXME: precise definition of the position.
    pub fn position(&self) -> f64 {
        self.position - WAVE_PAD_LEN as f64 + 2.0
    }

    /// Set the playback position.
    pub fn seek(&mut self, new_position: f64) {
        self.position = new_position + WAVE_PAD_LEN as f64 - 2.0;

        let ref clip = self.clip;
        let end_position = clip.num_samples() as f64 + WAVE_PAD_LEN as f64;
        let loop_len = if let Some(loop_start) = clip.loop_start() {
            Some((clip.num_samples() - loop_start) as f64)
        } else {
            None
        };

        if self.position >= end_position {
            if let Some(loop_len) = loop_len {
                while self.position >= end_position {
                    self.position -= loop_len;
                }
            } else {
                self.position = end_position;
            }
        }
    }

    pub fn pitch(&self) -> &DynamicValue {
        &self.pitch
    }

    pub fn pitch_mut(&mut self) -> &mut DynamicValue {
        &mut self.pitch
    }

    pub fn gain(&self) -> &DynamicValue {
        &self.gain
    }

    pub fn gain_mut(&mut self) -> &mut DynamicValue {
        &mut self.gain
    }

    pub fn is_stopped(&self) -> bool {
        !self.clip.is_looping() &&
            (self.position as usize) >= self.clip.num_samples() + WAVE_PAD_LEN
    }

    pub fn is_active(&self) -> bool {
        // The gain is non-zero
        !(self.gain.change_rate == 0f64 && self.gain.current == 0f64) &&

        // There is more samples to playback
        !self.is_stopped()
    }

    pub fn insert_event(&mut self, at: usize, event: &Event) {
        assert!(at < self.clip.num_samples());

        if self.cur_iter == u32::max_value() {
            // We should not use wrap-around for `iter`.
            // Instead, we shift all `iter` values
            // FIXME: exception safety
            let cur_iter = self.cur_iter;
            let new_events = self.events
                .drain()
                .map(|e| {
                    EventAssoc {
                        iter: e.iter - cur_iter,
                        ..e
                    }
                })
                .collect();
            self.events = new_events;
            self.cur_iter = 0;
        }

        let raw_pos = at as f64 + WAVE_PAD_LEN as f64;
        self.events.push(EventAssoc {
            iter: if self.position >= raw_pos {
                self.cur_iter + 1
            } else {
                self.cur_iter
            },
            position: raw_pos,
            event: event.clone(),
        });
    }

    /// Produce an output audio data.
    ///
    /// `to.len()` must be equal to `output_properties().num_channels`.
    pub fn render_additive(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        let ref clip = self.clip;
        let mut index = 0;
        let reader = clip.read_samples();
        let speed_scale = clip.sampling_rate() / self.output_prop.sampling_rate;
        let end_position = clip.num_samples() as f64 + WAVE_PAD_LEN as f64;
        let loop_len = if let Some(loop_start) = clip.loop_start() {
            Some((clip.num_samples() - loop_start) as f64)
        } else {
            None
        };

        macro_rules! case {
            ($num:expr, $has_events:expr) => (
                {
                    let mut writer = SliceZipMut::<[f32; $num], _, _>::new(to, range);
                    while index < writer.len() {
                        let remaining = writer.len() - index;

                        assert!(self.position >= 0.0);

                        if self.position >= end_position {
                            // Reached the end of clip.
                            self.pitch.update_multi(remaining as f64);
                            self.gain.update_multi(remaining as f64);
                            break;
                        }

                        let next_casp_time = self.pitch.next_cusp_time(remaining);
                        let next_casp_time = self.gain.next_cusp_time(next_casp_time);
                        debug_assert_ne!(next_casp_time, 0);
                        {
                            for _ in 0..next_casp_time {
                                // Generate the output waveform
                                {
                                    let mut out: [f32; $num] = writer.get(index).unwrap();
                                    let in_index = self.position as usize;
                                    debug_assert!(in_index + 3 < reader.num_samples() + WAVE_PAD_LEN * 2);
                                    let x = (self.position - self.position as usize as f64) as f32;
                                    let gain = self.gain.get() as f32;
                                    for i in 0..$num {
                                        let chan = unsafe { reader.raw_get_channel(i) };
                                        let in_samples = unsafe { [
                                            *chan.get_unchecked(in_index),
                                            *chan.get_unchecked(in_index + 1),
                                            *chan.get_unchecked(in_index + 2),
                                            *chan.get_unchecked(in_index + 3),
                                        ] };
                                        out[i] += cubic_hermite(x, in_samples) * gain;
                                    }
                                    writer.set(index, out);
                                }

                                self.position += speed_scale * self.pitch.get();

                                // Check events
                                if $has_events {
                                    loop {
                                        if let Some(event) = self.events.peek() {
                                            if event.iter == self.cur_iter && self.position >= event.position {
                                                event.event.set();
                                            } else {
                                                break;
                                            }
                                        } else {
                                            break;
                                        }
                                        self.events.pop().unwrap();
                                    }
                                }

                                if self.position >= end_position {
                                    if let Some(loop_len) = loop_len {
                                        if $has_events && self.events.peek().is_some() {
                                            self.cur_iter += 1;
                                        }

                                        // Loop
                                        self.position -= loop_len;

                                        // Check events again
                                        if $has_events {
                                            loop {
                                                if let Some(event) = self.events.peek() {
                                                    debug_assert_eq!(event.iter, self.cur_iter);
                                                    if self.position >= event.position {
                                                        event.event.set();
                                                    } else {
                                                        break;
                                                    }
                                                } else {
                                                    break;
                                                }
                                                self.events.pop();
                                            }
                                        }

                                        // Maybe the playback is really fast
                                        while self.position >= end_position {
                                            self.position -= loop_len;
                                        }
                                    } else {
                                        // Reached the end of clip.
                                        self.position = end_position;
                                        self.pitch.update();
                                        self.gain.update();
                                        index += 1;
                                        break;
                                    }
                                }

                                self.pitch.update();
                                self.gain.update();
                                index += 1;
                            }
                        }
                    }
                }
            )
        }
        match (to.len(), !self.events.is_empty()) {
            (1, false) => case!(1, false),
            (2, false) => case!(2, false),
            (3, false) => case!(3, false),
            (4, false) => case!(4, false),
            (5, false) => case!(5, false),
            (6, false) => case!(6, false),
            (7, false) => case!(7, false),
            (8, false) => case!(8, false),
            (1, true) => case!(1, true),
            (2, true) => case!(2, true),
            (3, true) => case!(3, true),
            (4, true) => case!(4, true),
            (5, true) => case!(5, true),
            (6, true) => case!(6, true),
            (7, true) => case!(7, true),
            (8, true) => case!(8, true),
            _ => panic!("too many channels"),
        }
    }
}

/// Perform Hermite cubic spline interpolation.
///
/// `samples` specifies the discrete sample values at `x = -1, 0, 1, 2`.
/// `x` must in the range `[0, 1]`.
fn cubic_hermite(x: f32, samples: [f32; 4]) -> f32 {
    let x2 = x * x;
    let x3 = x2 * x;
    samples[1] +
        (x * (samples[2] - samples[0]) +
             x2 * (samples[0] * 2f32 - samples[1] * 5f32 + samples[2] * 4f32 - samples[3]) +
             x3 * (samples[0] * -1f32 + samples[1] * 3f32 - samples[2] * 3f32 + samples[3])) *
            0.5f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cubic_hermite() {
        assert_eq!(cubic_hermite(0f32, [11f32, 4f32, 51f32, 4f32]), 4f32);
        assert_eq!(cubic_hermite(1f32, [11f32, 4f32, 51f32, 4f32]), 51f32);
    }
}
