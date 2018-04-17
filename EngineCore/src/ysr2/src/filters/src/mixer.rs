//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Mixer node.
use std::any::Any;
use ysr2_common::nodes::{Node, NodeInspector, NodeRenderContext, NodeId, OutputId};
use ysr2_common::values::DynamicValue;
use ysr2_common::utils::{IterablePool, PoolPtr};

/// Audio node that additively mixes multiple inputs and produces a single output.
///
/// # Node Properties
///
/// | # of inputs | # of outputs |
/// | ----------- | ------------ |
/// |   Dynamic   |       1      |
#[derive(Debug, Clone)]
pub struct MixerNode {
    sources: IterablePool<Source>,
}

/// Identifies a source in a `MixerNode`.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct SourceId(PoolPtr);

#[derive(Debug, Clone)]
struct Source {
    node_source: (NodeId, OutputId),
    gain: DynamicValue,
}

impl MixerNode {
    pub fn new() -> Self {
        Self { sources: IterablePool::new() }
    }

    pub fn insert_with_gain(&mut self, source: (NodeId, OutputId), gain: f64) -> SourceId {
        let source = Source {
            node_source: source,
            gain: DynamicValue::new(gain),
        };
        SourceId(self.sources.allocate(source))
    }

    pub fn insert(&mut self, source: (NodeId, OutputId)) -> SourceId {
        self.insert_with_gain(source, 1.0)
    }

    pub fn remove(&mut self, id: &SourceId) -> Option<(NodeId, OutputId)> {
        self.sources.deallocate(id.0).map(
            |source| source.node_source,
        )
    }

    pub fn gain(&self, id: &SourceId) -> Option<&DynamicValue> {
        self.sources.get(id.0).map(|src| &src.gain)
    }

    pub fn gain_mut(&mut self, id: &SourceId) -> Option<&mut DynamicValue> {
        self.sources.get_mut(id.0).map(|src| &mut src.gain)
    }
}

impl Node for MixerNode {
    fn num_outputs(&self) -> usize {
        1
    }

    fn inspect(&mut self, inspector: &mut NodeInspector) {
        for src in self.sources.iter() {
            inspector.declare_input(src.node_source).finish();
        }
    }

    fn render(&mut self, to: &mut [&mut [f32]], context: &NodeRenderContext) -> bool {
        assert_eq!(to.len(), 1);

        let ref mut to = to[0];
        let mut found_active = false;

        for src in self.sources.iter_mut() {
            let mut input = context.get_input(src.node_source).unwrap();
            if !input.is_active() {
                continue;
            }

            let samples = input.samples();
            let ref mut gain: DynamicValue = src.gain;
            if !found_active {
                if gain.is_stationary() {
                    let gain = gain.get() as f32;
                    if gain == 1.0 {
                        for (src, dst) in samples.iter().zip(to.iter_mut()) {
                            *dst = *src;
                        }
                    } else {
                        for (src, dst) in samples.iter().zip(to.iter_mut()) {
                            *dst = *src * gain;
                        }
                    }
                } else {
                    for (src, dst) in samples.iter().zip(to.iter_mut()) {
                        *dst = *src * gain.get() as f32;
                        gain.update();
                    }
                }
                found_active = true;
            } else {
                if gain.is_stationary() {
                    let gain = gain.get() as f32;
                    if gain == 1.0 {
                        for (src, dst) in samples.iter().zip(to.iter_mut()) {
                            *dst += *src;
                        }
                    } else {
                        for (src, dst) in samples.iter().zip(to.iter_mut()) {
                            *dst = src.mul_add(gain, *dst);
                        }
                    }
                } else {
                    for (src, dst) in samples.iter().zip(to.iter_mut()) {
                        *dst = src.mul_add(gain.get() as f32, *dst);
                        gain.update();
                    }
                }
            }
        }

        found_active
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}
