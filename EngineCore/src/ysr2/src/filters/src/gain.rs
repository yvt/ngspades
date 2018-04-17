//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Gain filter and node.
use std::any::Any;
use std::ops::Range;
use ysr2_common::nodes::{Node, NodeInspector, NodeRenderContext, NodeId, OutputId};
use ysr2_common::values::DynamicValue;
use {Filter, FilterNode};
use siso::SisoFilter;
use utils::apply_by_sample;

/// Gain filter.
#[derive(Debug, Clone)]
pub struct GainFilter {
    gain: DynamicValue,
}

impl GainFilter {
    /// Construct a new `GainFilter` with the gain value `1.0`.
    pub fn new() -> Self {
        Self::with_gain(1.0)
    }

    /// Construct a new `GainFilter` with the specified gain.
    pub fn with_gain(gain: f64) -> Self {
        Self { gain: DynamicValue::new(gain) }
    }
}

impl SisoFilter for GainFilter {
    fn num_channels(&self) -> Option<usize> {
        None
    }
}

impl Filter for GainFilter {
    fn render(
        &mut self,
        to: &mut [&mut [f32]],
        range: Range<usize>,
        from: Option<(&[&[f32]], Range<usize>)>,
    ) {
        // validate the range
        assert!(range.start <= range.end);
        for ch in to.iter() {
            let _ = &ch[range.clone()];
        }

        for i in 0..to.len() {
            let mut gain = self.gain.clone();
            apply_by_sample(
                &mut to[i][range.clone()],
                from.as_ref().map(|&(ref inputs, ref in_range)| &inputs[i][in_range.clone()]),
                move |iter| {
                    if gain.is_stationary() {
                        for x in iter {
                            *x *= gain.get() as f32;
                        }
                    } else {
                        for x in iter {
                            *x *= gain.get() as f32;
                            gain.update();
                        }
                    }
                },
            );
        }

        self.gain.update_multi(range.len() as f64);
    }

    fn is_active(&self) -> bool {
        false
    }

    fn num_input_channels(&self) -> Option<usize> {
        self.num_channels()
    }

    fn num_output_channels(&self) -> Option<usize> {
        self.num_channels()
    }

    fn skip(&mut self, _: usize) {}

    fn reset(&mut self) {}
}

/// Gain filter node.
///
/// # Node Properties
///
/// | # of inputs | # of outputs |
/// | ----------- | ------------ |
/// |      1      |       1      |
#[derive(Debug, Clone)]
pub struct GainNode(FilterNode<GainFilter>);

impl GainNode {
    /// Constructs a `GainNode` with the gain value `1.0`.
    pub fn new() -> Self {
        GainNode(FilterNode::new(GainFilter::new(), 1, 1))
    }

    /// Constructs a `GainNode` with the specified gain.
    pub fn with_gain(gain: f64) -> Self {
        GainNode(FilterNode::new(GainFilter::with_gain(gain), 1, 1))
    }

    /// Get a reference to the source of the specified input.
    pub fn input_source(&self) -> &Option<(NodeId, OutputId)> {
        self.0.input_source(0).unwrap()
    }

    /// Get a mutable reference to the source of the specified input.
    pub fn input_source_mut(&mut self) -> &mut Option<(NodeId, OutputId)> {
        self.0.input_source_mut(0).unwrap()
    }
}

impl Node for GainNode {
    fn num_outputs(&self) -> usize {
        self.0.num_outputs()
    }

    fn inspect(&mut self, inspector: &mut NodeInspector) {
        self.0.inspect(inspector)
    }

    fn render(&mut self, to: &mut [&mut [f32]], context: &NodeRenderContext) -> bool {
        self.0.render(to, context)
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}
