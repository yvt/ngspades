//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::any::Any;
use ysr2_common::nodes::{Node, NodeInspector, NodeRenderContext, NodeId, OutputId};
use {Filter, FilterNode};
use biquad::{SimpleBiquadKernel, BiquadCoefs};

/// Bi-quad filter node.
///
/// # Node Properties
///
/// | # of inputs | # of outputs |
/// | ----------- | ------------ |
/// |      1      |       1      |
#[derive(Debug, Clone)]
pub struct BiquadNode(FilterNode<SimpleBiquadKernel>);

impl BiquadNode {
    /// Constructs a `BiquadNode`.
    pub fn new(coefs: &BiquadCoefs) -> Self {
        BiquadNode(FilterNode::new(SimpleBiquadKernel::new(coefs, 1), 1, 1))
    }

    /// Reset the filter to the stasis state.
    pub fn reset(&mut self) {
        self.0.get_ref_mut().reset();
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

impl Node for BiquadNode {
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
