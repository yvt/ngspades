//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::any::Any;
use std::ops::Range;
use ysr2_common::nodes::{Node, NodeInspector, NodeRenderContext, NodeId, OutputId};
use {Filter, FilterNode};
use siso::SisoFilter;
use utils::apply_by_sample;

#[derive(Debug, Clone)]
pub struct DelayFilter {
    states: Vec<Vec<f32>>,
    position: usize,
    left_samples: usize,
}

impl DelayFilter {
    pub fn new(latency: usize, num_channels: usize) -> Self {
        assert_ne!(num_channels, 0);
        Self {
            states: vec![vec![0.0; latency]; num_channels],
            position: 0,
            left_samples: 0,
        }
    }
}

impl SisoFilter for DelayFilter {
    fn num_channels(&self) -> Option<usize> {
        Some(self.states.len())
    }
}

impl Filter for DelayFilter {
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
        assert_eq!(self.states.len(), to.len());

        let mut max_intensity = 0f32;

        for i in 0..to.len() {
            let ref mut state = self.states[i];
            let mut position = self.position;
            apply_by_sample(
                &mut to[i][range.clone()],
                from.as_ref().map(|&(ref inputs, ref in_range)| &inputs[i][in_range.clone()]),
                |iter| {
                    use std::mem::swap;

                    let st = &mut state[..];
                    assert!(position < st.len());

                    let mut max_int = 0f32;

                    for x in iter {
                        max_int = max_int.max(x.abs());

                        swap(x, &mut st[position]);
                        position += 1;
                        if position >= st.len() {
                            position = 0;
                        }
                    }

                    max_intensity = max_intensity.max(max_int);
                },
            );
        }

        self.position = (self.position + range.len()) % self.states[0].len();

        if max_intensity > 1.0e-8 {
            self.left_samples = self.states[0].len();
        }
    }

    fn is_active(&self) -> bool {
        self.left_samples > 0
    }

    fn num_input_channels(&self) -> Option<usize> {
        self.num_channels()
    }

    fn num_output_channels(&self) -> Option<usize> {
        self.num_channels()
    }

    fn skip(&mut self, num_samples: usize) {
        if self.left_samples == 0 {
            return;
        }

        let latency = self.states[0].len();
        if num_samples >= latency {
            self.reset();
        } else {
            let pos = self.position;
            if latency - pos > num_samples {
                for x in self.states.iter_mut() {
                    for y in &mut x[pos..pos + num_samples] {
                        *y = 0.0;
                    }
                }
                self.position = pos + num_samples;
            } else {
                for x in self.states.iter_mut() {
                    for y in &mut x[pos..] {
                        *y = 0.0;
                    }
                    for y in &mut x[0..num_samples - (latency - pos)] {
                        *y = 0.0;
                    }
                }
                self.position = num_samples - (latency - pos);
            }
        }

        self.left_samples = self.left_samples.saturating_sub(num_samples);
    }

    fn reset(&mut self) {
        if self.left_samples == 0 {
            return;
        }

        for x in self.states.iter_mut() {
            for y in x.iter_mut() {
                *y = 0.0;
            }
        }

        self.left_samples = 0;
    }
}

/// Delay node.
///
/// # Node Properties
///
/// | # of inputs | # of outputs |
/// | ----------- | ------------ |
/// |      1      |       1      |
#[derive(Debug, Clone)]
pub struct DelayNode(FilterNode<DelayFilter>);

impl DelayNode {
    /// Constructs a `DelayNode`.
    pub fn new(latency: usize) -> Self {
        DelayNode(FilterNode::new(DelayFilter::new(latency, 1), 1, 1))
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

impl Node for DelayNode {
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
