//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! A chunk of audio data.
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::Arc;
use ysr2_common::stream::StreamProperties;

pub(crate) const WAVE_PAD_LEN: usize = 4;

/// A chunk of audio data.
#[derive(Debug, Clone)]
pub struct Clip(Arc<ClipData>);

/// RAII structure used to read the waveform of `Clip`.
pub struct ClipReadGuard<'a>(RwLockReadGuard<'a, Vec<Vec<f32>>>, usize);

/// RAII structure used to read and write the waveform of `Clip`.
pub struct ClipWriteGuard<'a>(RwLockWriteGuard<'a, Vec<Vec<f32>>>, usize, Option<usize>);

#[derive(Debug)]
struct ClipData {
    wave: RwLock<Vec<Vec<f32>>>,
    prop: StreamProperties,
    num_samples: usize,
    loop_start: Option<usize>,
}

impl Clip {
    pub fn new(num_samples: usize, loop_start: Option<usize>, prop: &StreamProperties) -> Self {
        if let Some(loop_start) = loop_start {
            assert!(
                loop_start < num_samples,
                "loop_start must be less than num_samples"
            );
        }

        // We could get into a trouble if the number of samples is so high that
        // it cannot be represented accurately in `f64`
        assert!((num_samples as u64) < 0x1000000000000, "too many samples");

        let data = ClipData {
            wave: RwLock::new(
                (0..prop.num_channels)
                    .map(|_| vec![0f32; num_samples + WAVE_PAD_LEN * 2])
                    .collect(),
            ),
            prop: prop.clone(),
            num_samples,
            loop_start,
        };
        Clip(Arc::new(data))
    }

    pub fn read_samples(&self) -> ClipReadGuard {
        ClipReadGuard(self.0.wave.read(), self.0.num_samples)
    }

    pub fn write_samples(&self) -> ClipWriteGuard {
        ClipWriteGuard(self.0.wave.write(), self.0.num_samples, self.0.loop_start)
    }

    pub fn stream_properties(&self) -> &StreamProperties {
        &self.0.prop
    }

    pub fn sampling_rate(&self) -> f64 {
        self.0.prop.sampling_rate
    }

    pub fn num_channels(&self) -> usize {
        self.0.prop.num_channels
    }

    pub fn num_samples(&self) -> usize {
        self.0.num_samples
    }

    pub fn is_looping(&self) -> bool {
        self.loop_start().is_some()
    }

    pub fn loop_start(&self) -> Option<usize> {
        self.0.loop_start
    }
}

impl<'a> ClipReadGuard<'a> {
    pub fn get_channel(&self, channel: usize) -> &[f32] {
        &self.0[channel][WAVE_PAD_LEN..WAVE_PAD_LEN + self.1]
    }

    pub(crate) fn raw_get_channel(&self, channel: usize) -> &[f32] {
        &self.0[channel][..]
    }

    pub fn num_channels(&self) -> usize {
        self.0.len()
    }

    pub fn num_samples(&self) -> usize {
        self.1
    }
}

impl<'a> ClipWriteGuard<'a> {
    pub fn get_channel(&self, channel: usize) -> &[f32] {
        &self.0[channel][WAVE_PAD_LEN..WAVE_PAD_LEN + self.1]
    }

    pub fn get_channel_mut(&mut self, channel: usize) -> &mut [f32] {
        &mut self.0[channel][WAVE_PAD_LEN..WAVE_PAD_LEN + self.1]
    }

    pub fn num_channels(&self) -> usize {
        self.0.len()
    }

    pub fn num_samples(&self) -> usize {
        self.1
    }
}

impl<'a> Drop for ClipWriteGuard<'a> {
    fn drop(&mut self) {
        if let Some(loop_start) = self.2 {
            // This is a looping audio clip.
            // Update the pad.
            let num_samples = self.1;
            let loop_len = num_samples - loop_start;
            for ch in self.0.iter_mut() {
                for i in num_samples + WAVE_PAD_LEN..ch.len() {
                    ch[i] = ch[i - loop_len];
                }
            }
        }
    }
}
