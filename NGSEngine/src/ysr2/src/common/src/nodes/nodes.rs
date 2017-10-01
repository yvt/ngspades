//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::any::Any;
use nodes::{Node, NodeInspector, NodeRenderContext, NodeId, OutputId};

/// Audio output node.
#[derive(Debug)]
pub struct OutputNode {
    inputs: Vec<Input>,
    filled: bool,
}

#[derive(Default, Debug, Clone)]
struct Input {
    buffer: Vec<f32>,
    source: Option<(NodeId, OutputId)>,
}

impl OutputNode {
    /// Constructs a new `OutputNode`.
    ///
    /// `num_inputs` must not be `0`.
    pub fn new(num_inputs: usize) -> Self {
        assert_ne!(num_inputs, 0);
        Self {
            inputs: vec![Default::default(); num_inputs],
            filled: false,
        }
    }

    /// Allocate buffers to accept `num_samples` samples in the next frame.
    pub fn request_frame(&mut self, num_samples: usize) {
        self.filled = false;
        for input in self.inputs.iter_mut() {
            input.buffer.resize(num_samples, 0.0);
        }
    }

    /// Get a reference to the source of the specified input.
    pub fn input_source(&self, input_index: usize) -> Option<&Option<(NodeId, OutputId)>> {
        self.inputs.get(input_index).map(|input| &input.source)
    }

    /// Get a mutable reference to the source of the specified input.
    pub fn input_source_mut(
        &mut self,
        input_index: usize,
    ) -> Option<&mut Option<(NodeId, OutputId)>> {
        self.inputs.get_mut(input_index).map(
            |input| &mut input.source,
        )
    }

    /// Get the numboer of inputs.
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }

    /// Get the receive buffer for the specified input.
    pub fn get_samples(&self, input_index: usize) -> Option<&[f32]> {
        self.inputs.get(input_index).map(
            |input| input.buffer.as_slice(),
        )
    }
}

impl Node for OutputNode {
    fn num_outputs(&self) -> usize {
        0
    }

    fn inspect(&mut self, scanner: &mut NodeInspector) {
        if self.filled {
            return;
        }
        for input in self.inputs.iter() {
            if let Some(source) = input.source {
                scanner
                    .declare_input(source)
                    .num_samples(input.buffer.len())
                    .finish();
            }
        }
    }

    fn render(&mut self, to: &mut [&mut [f32]], context: &NodeRenderContext) -> bool {
        assert_eq!(to.len(), 0);

        for input in self.inputs.iter_mut() {
            if let Some(source) = input.source {
                let mut node_input = context.get_input(source).unwrap();
                let source_buf = node_input.samples();
                input.buffer.copy_from_slice(source_buf);
            } else {
                for x in input.buffer.iter_mut() {
                    *x = 0.0;
                }
            }
        }

        self.filled = true;
        false
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}

/// Node that outputs zero.
///
/// This node has a single output channel whose `OutputId` is `0`.
#[derive(Debug)]
pub struct ZeroNode;

impl Node for ZeroNode {
    fn num_outputs(&self) -> usize {
        1
    }

    fn inspect(&mut self, _: &mut NodeInspector) {}

    fn render(&mut self, _: &mut [&mut [f32]], _: &NodeRenderContext) -> bool {
        false
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}
