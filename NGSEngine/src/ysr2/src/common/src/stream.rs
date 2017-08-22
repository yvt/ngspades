//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;
use std::borrow::BorrowMut;

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

    /// Produce an audio stream but do not write the output anywhere.
    ///
    /// This can be called only if `is_active()` is `false`.
    fn skip(&mut self, num_samples: usize);

    /// Determine whether a following call to `render` has a possibility to
    /// produce non-zero audio data or not.
    fn is_active(&self) -> bool { true }
}

impl<T: Generator> Generator for BorrowMut<T> {
    fn render(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        self.borrow_mut().render(to, range)
    }

    fn skip(&mut self, num_samples: usize) {
        self.borrow_mut().skip(num_samples);
    }

    fn is_active(&self) -> bool {
        self.borrow().is_active()
    }
}