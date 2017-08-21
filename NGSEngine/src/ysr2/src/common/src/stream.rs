//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;

#[derive(Debug, PartialEq, Clone)]
pub struct StreamProperties {
    pub sampling_rate: f64,
    pub num_channels: usize,
    pub channel_config: ChannelConfig,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ChannelConfig {
    /// The monaural configuration.
    ///
    /// The number of channel must be one.
    Monaural,

    /// The stereo configuration.
    ///
    /// The number of channel must be two.
    Stereo,

    /// Generic configuration.
    Generic,
}

pub trait Generator {
    /// Produce audio data.
    fn render(&mut self, to: &mut [&mut [f32]], range: Range<usize>);

    /// Determine whether a following call to `render` has a possibility to
    /// produce non-zero audio data or not.
    fn is_active(&self) -> bool { true }
}