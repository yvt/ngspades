//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![cfg_attr(test, feature(test))]
extern crate ysr2_common;
extern crate primal;
extern crate arrayvec;
extern crate yfft;

use std::any::Any;
use std::fmt::Debug;
use std::ops::Range;
use arrayvec::ArrayVec;
use ysr2_common::nodes::{Node, NodeInspector, NodeRenderContext, NodeId, OutputId};

pub mod biquad;
pub mod conv;
pub mod reverb;
pub mod siso;
mod utils;

/// A causal filter.
pub trait Filter {
    /// Apply the filter to the input signal `from.unwrap_or((to, range))` and
    /// write the output to `to`.
    ///
    /// - If `from` is `Some((inputs, from_range))`, `from_range.len()` must be
    ///   equal to `range.len()` and `inputs.len()` must be equal to `to.len()`.
    /// - If `num_output_channels()` is `Some(x)`, then `to.len()` must be equal
    ///   to `x`.
    /// - If `num_input_channels()` is `Some(x)`, then
    ///   `from.unwrap_or((to, range)).0.len()` must be equal to `x`.
    ///
    fn render(
        &mut self,
        to: &mut [&mut [f32]],
        range: Range<usize>,
        from: Option<(&[&[f32]], Range<usize>)>,
    );

    /// Apply the filter to the signal `to` in-place. Can be used only if
    /// `num_input_channels() == num_output_channels()`.
    ///
    /// This can be used a syntax sugar of `render(to, range, None)`.
    fn render_inplace(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        self.render(to, range, None)
    }

    /// Return the number of channels of the input signal.
    ///
    /// `None` indicates the value is not restricted, or governed by some
    /// other restrictions.
    fn num_input_channels(&self) -> Option<usize>;

    /// Return the number of channels of the output signal.
    ///
    /// `None` indicates the value is not restricted, or governed by some
    /// other restrictions.
    fn num_output_channels(&self) -> Option<usize>;

    /// Determine whether a following call to `render` generates a non-zero
    /// (more precisely, above a predetermined threshold) signal even with a
    /// zero input signal.
    fn is_active(&self) -> bool;

    /// Feed `num_samples` samples with zero values and discard the output.
    fn skip(&mut self, num_samples: usize);

    /// Reset the filter to the initial state.
    fn reset(&mut self);
}

/// `Node` wrapper for `Filter`.
#[derive(Debug, Clone)]
pub struct FilterNode<T> {
    filter: T,
    num_outputs: usize,
    inputs: Vec<Option<(NodeId, OutputId)>>,
}

impl<T> FilterNode<T> {
    /// Constructs a `FilterNode`.
    ///
    /// `num_outputs` and `num_inputs` must not be zero.
    ///
    /// Restriction due to the current implementation: `num_inputs` must be
    /// less than or equal to `64`.
    pub fn new(x: T, num_inputs: usize, num_outputs: usize) -> Self {
        assert_ne!(num_outputs, 0);
        assert_ne!(num_inputs, 0);
        Self {
            filter: x,
            num_outputs,
            inputs: vec![None; num_inputs],
        }
    }

    /// Get a reference to the underlying filter.
    pub fn get_ref(&self) -> &T {
        &self.filter
    }

    /// Get a mutable reference to the underlying filter.
    pub fn get_ref_mut(&mut self) -> &mut T {
        &mut self.filter
    }

    /// Unwrap this `FilterNode`, returning the underlying filter.
    pub fn into_inner(self) -> T {
        self.filter
    }

    /// Get a reference to the source of the specified input.
    pub fn input_source(&self, input_index: usize) -> Option<&Option<(NodeId, OutputId)>> {
        self.inputs.get(input_index)
    }

    /// Get a mutable reference to the source of the specified input.
    pub fn input_source_mut(
        &mut self,
        input_index: usize,
    ) -> Option<&mut Option<(NodeId, OutputId)>> {
        self.inputs.get_mut(input_index)
    }

    /// Get the numboer of outputs.
    pub fn num_outputs(&self) -> usize {
        self.num_outputs
    }

    /// Get the numboer of inputs.
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }
}

impl<T> Node for FilterNode<T>
where
    T: Filter + Debug + Sync + Send + 'static,
{
    fn num_outputs(&self) -> usize {
        self.num_outputs
    }

    fn inspect(&mut self, inspector: &mut NodeInspector) {
        for input in self.inputs.iter() {
            let source = input.expect("FilterNode has an unconnected input");
            inspector.declare_input(source).finish();
        }
    }

    fn render(&mut self, to: &mut [&mut [f32]], context: &NodeRenderContext) -> bool {
        let num_samples = to[0].len();

        if !self.filter.is_active() {
            let mut found_active_in = false;
            for input in self.inputs.iter() {
                let source = input.unwrap();
                if context.get_input(source).unwrap().is_active() {
                    found_active_in = true;
                    break;
                }
            }
            if !found_active_in {
                self.filter.skip(num_samples);
                return false;
            }
        }

        let mut inputs: ArrayVec<[_; 64]> = self.inputs
            .iter()
            .map(|input| {
                let source = input.unwrap();
                context.get_input(source).unwrap()
            })
            .collect();
        let input_samples: ArrayVec<[_; 64]> =
            inputs.iter_mut().map(|input| input.samples()).collect();

        let num_samples = to[0].len();
        self.filter.render(
            to,
            0..num_samples,
            Some((&input_samples[..], 0..num_samples)),
        );
        true
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}
