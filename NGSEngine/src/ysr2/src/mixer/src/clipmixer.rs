//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Manages the playback of multiple `Clip`s and produces an audio stream using
//! `ClipPlayer` associated with each played note.
use std::collections::{BinaryHeap, HashMap};
use std::cmp;
use std::ops::Range;

use clip::Clip;
use clipplayer::ClipPlayer;
use ysr2_common::stream::{Generator, StreamProperties};

/// Manages the playback of multiple `Clip`s and produces an audio stream using
/// `ClipPlayer` associated with each played note.
#[derive(Debug)]
pub struct ClipMixer {
    notes: HashMap<NoteId, Note>,
    output_prop: StreamProperties,
    time: u64,
    next_id: NoteId,
}

/// An identifier for a single note to be played by `ClipMixer`.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct NoteId(u64);

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum NoteError {
    /// Could not find a note with the identifier `NoteId`.
    ///
    /// The following list shows the possible reasons:
    ///
    ///  - The `NoteId` originates from a different `ClipMixer`.
    ///  - There used to be a note with the `NoteId`, whose playback has already
    ///    been completed.
    ///
    UnknownId,
}

#[derive(Debug)]
struct Note {
    player: ClipPlayer,
    started: bool,
    stopped: bool,
    events: BinaryHeap<NoteEvent>,
}

#[derive(Debug)]
struct NoteEvent {
    abs_time: u64,
    data: NoteEventData,
}

impl Ord for NoteEvent {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.abs_time.cmp(&other.abs_time).reverse()
    }
}
impl PartialOrd for NoteEvent {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for NoteEvent {
    fn eq(&self, other: &Self) -> bool {
        self.abs_time == other.abs_time
    }
}
impl Eq for NoteEvent {}

#[derive(Debug, Clone)]
enum NoteEventData {
    Start,
    Stop,
    Seek(f64),
    SetGain(f64, f64),
    SetPitch(f64, f64),
}

#[derive(Debug)]
pub struct NoteBuilder<'a> {
    mixer: &'a mut ClipMixer,
    note: Note,
}

impl ClipMixer {
    // TODO: ClipMixer

    pub fn new(output_prop: &StreamProperties) -> Self {
        Self {
            notes: HashMap::new(),
            output_prop: output_prop.clone(),
            time: 0,
            next_id: NoteId(0),
        }
    }

    /// Retrieve a reference to the `ClipPlayer` associated with the given
    /// `NoteId`.
    pub fn get_player(&self, id: NoteId) -> Result<&ClipPlayer, NoteError> {
        self.notes.get(&id).map(|note| &note.player).ok_or(
            NoteError::UnknownId,
        )
    }

    /// Retrieve a mutable reference to the `ClipPlayer` associated with the
    /// given `NoteId`.
    pub fn get_player_mut(&mut self, id: NoteId) -> Result<&mut ClipPlayer, NoteError> {
        self.notes.get_mut(&id).map(|note| &mut note.player).ok_or(
            NoteError::UnknownId,
        )
    }

    /// Constructs a `NoteBuilder` that can be used to start playing a `Clip`
    /// using this `ClipMixer`.
    ///
    /// You must call the returned `NoteBuilder`'s `start` to start the playback.
    /// Before doing that, you can call methods provided by `NoteBuilder` to
    /// configure various properties.
    pub fn build_note(&mut self, clip: &Clip) -> NoteBuilder {
        let note = Note {
            player: ClipPlayer::new(clip, &self.output_prop),
            started: false,
            stopped: false,
            events: BinaryHeap::new(),
        };

        NoteBuilder { mixer: self, note }
    }

    /// Adjust the internal state so `self.time` becomes zero.
    ///
    /// It is an extremely rare case that this method is called, because time
    /// values are represented using `u64` and it takes about 3 million years to
    /// overflow a time value even if the sample rate is set to 176400 hertz.
    fn reset_time_base(&mut self) {
        let now = self.time;
        if now == 0 {
            return;
        }

        // This is not exception-safe, unfortunately
        for (_, note) in self.notes.iter_mut() {
            let new_events = note.events
                .iter()
                .map(|event| {
                    NoteEvent {
                        abs_time: event.abs_time - now,
                        data: event.data.clone(),
                    }
                })
                .collect();
            note.events = new_events;
        }

        self.time = 0;
    }

    fn time_rel_to_abs(&mut self, rel: u64) -> u64 {
        if let Some(abs_time) = self.time.checked_add(rel) {
            abs_time
        } else {
            self.reset_time_base();
            rel
        }
    }

    fn allocate_id(&mut self) -> NoteId {
        let id = self.next_id;
        self.next_id = NoteId(id.0.checked_add(1).unwrap());
        id
    }

    fn push_note_event(
        &mut self,
        delay: u64,
        id: NoteId,
        data: NoteEventData,
    ) -> Result<(), NoteError> {
        let abs_time = self.time_rel_to_abs(delay);

        if let Some(note) = self.notes.get_mut(&id) {
            note.events.push(NoteEvent { abs_time, data });
            Ok(())
        } else {
            Err(NoteError::UnknownId)
        }
    }

    /// Stop a note after a specified duration (measured in samples).
    pub fn stop(&mut self, delay: u64, id: NoteId) -> Result<(), NoteError> {
        self.push_note_event(delay, id, NoteEventData::Stop)
    }

    /// Modify the playback positiion of a note after a specified duration
    /// (measured in samples).
    pub fn seek(&mut self, delay: u64, id: NoteId, new_position: f64) -> Result<(), NoteError> {
        self.push_note_event(delay, id, NoteEventData::Seek(new_position))
    }

    /// Set the gain of a note after a duration specified by `delay` (measured
    /// in samples).
    ///
    /// The change will occur gradually during the period specified by `duration`
    /// (measured in samples). `duration` must be a non-negative finite value.
    pub fn set_gain(
        &mut self,
        delay: u64,
        id: NoteId,
        new_gain: f64,
        duration: f64,
    ) -> Result<(), NoteError> {
        self.push_note_event(delay, id, NoteEventData::SetGain(new_gain, duration))
    }

    /// Set the pitch of a note after a duration specified by `delay` (measured
    /// in samples).
    ///
    /// The change will occur gradually during the period specified by `duration`
    /// (measured in samples). `duration` must be a non-negative finite value.
    pub fn set_pitch(
        &mut self,
        delay: u64,
        id: NoteId,
        new_pitch: f64,
        duration: f64,
    ) -> Result<(), NoteError> {
        self.push_note_event(delay, id, NoteEventData::SetPitch(new_pitch, duration))
    }
}

impl Generator for ClipMixer {
    fn render(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        for series in to.iter_mut() {
            for sample in &mut series[range.clone()] {
                *sample = 0.0;
            }
        }

        assert!(range.start <= range.end);

        let num_rendered_samples: usize = range.len();
        let new_cur_time: u64 = self.time_rel_to_abs(num_rendered_samples as u64);

        for (_, note) in self.notes.iter_mut() {
            let mut time: u64 = self.time;
            let mut rel_time: usize = 0;

            while rel_time < num_rendered_samples {
                let (slice_time, has_event) = if let Some(event) = note.events.peek() {
                    let delta = event.abs_time - time;
                    if delta > (num_rendered_samples - rel_time) as u64 {
                        (num_rendered_samples - rel_time, false)
                    } else {
                        (delta as usize, true)
                    }
                } else {
                    (num_rendered_samples - rel_time, false)
                };

                if slice_time > 0 {
                    if note.started {
                        note.player.render_additive(
                            to,
                            rel_time + range.start..rel_time + range.start + slice_time,
                        );
                    } else {
                        note.player.pitch_mut().update_multi(slice_time as f64);
                        note.player.gain_mut().update_multi(slice_time as f64);
                    }
                }

                time += slice_time as u64;
                rel_time += slice_time;

                if has_event {
                    let event = note.events.pop().unwrap();
                    assert_eq!(event.abs_time, time);
                    match event.data {
                        NoteEventData::Start => {
                            note.started = true;
                        }
                        NoteEventData::Stop => {
                            note.stopped = true;
                        }
                        NoteEventData::Seek(time) => {
                            note.player.seek(time);
                        }
                        NoteEventData::SetGain(value, duration) => {
                            note.player.gain_mut().set_slow(value, duration);
                        }
                        NoteEventData::SetPitch(value, duration) => {
                            note.player.pitch_mut().set_slow(value, duration);
                        }
                    }
                }

                if note.stopped {
                    break;
                }
            }

            assert!(note.stopped || time == new_cur_time);
        }

        self.notes.retain(|_, note| {
            !note.stopped && !note.player.is_stopped()
        });
        self.time = new_cur_time;
    }

    fn is_active(&self) -> bool {
        !self.notes.is_empty()
    }
}

impl<'a> NoteBuilder<'a> {
    /// Start the playback of the note after a specified duration.
    pub fn start(mut self, delay: u64) -> NoteId {
        if delay == 0 {
            self.note.started = true;
        } else {
            self.note.events.push(NoteEvent {
                abs_time: self.mixer.time_rel_to_abs(delay),
                data: NoteEventData::Start,
            });
        }

        let id = self.mixer.allocate_id();
        assert!(self.mixer.notes.insert(id, self.note).is_none());
        id
    }

    pub fn gain(mut self, gain: f64) -> Self {
        self.note.player.gain_mut().set(gain);
        self
    }

    pub fn pitch(mut self, pitch: f64) -> Self {
        self.note.player.pitch_mut().set(pitch);
        self
    }
}
