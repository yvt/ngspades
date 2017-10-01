//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::fmt::Debug;
use std::ops::Range;
use std::borrow::BorrowMut;
use std::any::Any;
use nodes::{Node, NodeInspector, NodeRenderContext};

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
    fn is_active(&self) -> bool {
        true
    }
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

pub struct ZeroGenerator;

impl Generator for ZeroGenerator {
    fn render(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        for ch in to.iter_mut() {
            for x in ch[range.clone()].iter_mut() {
                *x = 0.0;
            }
        }
    }

    fn skip(&mut self, _: usize) {}

    fn is_active(&self) -> bool {
        false
    }
}

/// `Node` wrapper for `Generator`.
#[derive(Debug)]
pub struct GeneratorNode<T> {
    generator: T,
    num_outputs: usize,
}

impl<T> GeneratorNode<T> {
    /// Constructs a `GeneratorNode`.
    ///
    /// `num_outputs` must not be zero.
    pub fn new(x: T, num_outputs: usize) -> Self {
        assert_ne!(num_outputs, 0);
        Self {
            generator: x,
            num_outputs,
        }
    }

    /// Get a reference to the underlying generator.
    pub fn get_ref(&self) -> &T {
        &self.generator
    }

    /// Get a mutable reference to the underlying generator.
    pub fn get_ref_mut(&mut self) -> &mut T {
        &mut self.generator
    }

    /// Unwrap this `GeneratorNode`, returning the underlying generator.
    pub fn into_inner(self) -> T {
        self.generator
    }
}

impl<T> Node for GeneratorNode<T>
where
    T: Generator + Debug + Sync + Send + 'static,
{
    fn num_outputs(&self) -> usize {
        self.num_outputs
    }

    fn inspect(&mut self, _: &mut NodeInspector) {}

    fn render(&mut self, to: &mut [&mut [f32]], _: &NodeRenderContext) -> bool {
        let num_samples = to[0].len();
        if self.generator.is_active() {
            self.generator.render(to, 0..num_samples);
            true
        } else {
            self.generator.skip(num_samples);
            false
        }
    }

    fn as_any(&self) -> &Any { self }

    fn as_any_mut(&mut self) -> &mut Any { self }
}
