//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::any::Any;
use ysr2_common::nodes::{Node, NodeInspector, NodeRenderContext, NodeId, OutputId};
use {Filter, FilterNode};
use reverb::{MatrixReverb, MatrixReverbParams};

/// Node version of `MatrixReverb`.
///
/// # Node Properties
///
/// | # of inputs | # of outputs |
/// | ----------- | ------------ |
/// |      1      |     1 – 8    |
#[derive(Debug, Clone)]
pub struct MatrixReverbNode(FilterNode<MatrixReverb>);

impl MatrixReverbNode {
    /// Constructs a `MatrixReverbNode`.
    ///
    /// `num_output_channels` must be in the range `[1, 8]`.
    pub fn new(params: &MatrixReverbParams, num_output_channels: usize) -> Self {
        assert!(num_output_channels <= 8);
        MatrixReverbNode(FilterNode::new(
            MatrixReverb::new(params),
            1,
            num_output_channels,
        ))
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

impl Node for MatrixReverbNode {
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
