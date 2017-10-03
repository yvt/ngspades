//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::fmt::Debug;
use std::any::Any;
use ysr2_common::nodes::{Node, NodeInspector, NodeRenderContext, NodeId, OutputId,
    NodeInputGenerator, NodeInputGeneratorHost};
use ysr2_common::utils::{IterablePool, PoolPtr};
use ysr2_common::values::DynamicSlerpVector3;
use Panner;

/// `Node` wrapper of `Panner`.
///
/// # Node Properties
///
/// | # of inputs | # of outputs |
/// | ----------- | ------------ |
/// |   Dynamic   |    Varies    |
#[derive(Debug)]
pub struct PannerNode<T: Panner<NodeInputGenerator>> {
    panner: T,
    num_outputs: usize,
    sources: IterablePool<Source<T::SourceId>>,
    host: NodeInputGeneratorHost,
}

/// Identifies a source in a `PannerNode`.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct SourceId(PoolPtr);

#[derive(Debug)]
struct Source<T> {
    inner_id: T,
}

impl<T: Panner<NodeInputGenerator>> PannerNode<T> {
    /// Constructs a `PannerNode`.
    ///
    /// `num_outputs` must not be zero.
    pub fn new(x: T, num_outputs: usize) -> Self {
        assert_ne!(num_outputs, 0);
        Self {
            panner: x,
            num_outputs,
            sources: IterablePool::new(),
            host: NodeInputGeneratorHost::new(),
        }
    }

    /// Get a reference to the underlying panner.
    pub fn get_ref(&self) -> &T {
        &self.panner
    }

    /// Get a mutable reference to the underlying panner.
    pub fn get_ref_mut(&mut self) -> &mut T {
        &mut self.panner
    }

    /// Unwrap this `PannerNode`, returning the underlying panner.
    pub fn into_inner(self) -> T {
        self.panner
    }

    pub fn insert(&mut self, source: (NodeId, OutputId)) -> SourceId {
        let mut gen = self.host.make_generator(1);
        *gen.input_source_mut(0).unwrap() = Some(source);

        let inner_id = self.panner.insert(gen);
        SourceId(self.sources.allocate(Source { inner_id }))
    }

    pub fn remove(&mut self, id: &SourceId) -> Option<Option<(NodeId, OutputId)>> {
        self.sources.deallocate(id.0)
            .map(|source| self.panner.remove(&source.inner_id).unwrap())
            .map(|gen| *gen.input_source(0).unwrap())
    }

    pub fn direction(&self, id: &SourceId) -> Option<&DynamicSlerpVector3> {
        let ref panner = self.panner;
        self.sources.get(id.0).map(move |src| panner.direction(&src.inner_id).unwrap())
    }

    pub fn direction_mut(&mut self, id: &SourceId) -> Option<&mut DynamicSlerpVector3> {
        let ref mut panner = self.panner;
        self.sources.get(id.0).map(move |src| panner.direction_mut(&src.inner_id).unwrap())
    }

    /// Get a reference to the source of the specified source.
    pub fn input_source(&self, id: &SourceId) -> Option<&Option<(NodeId, OutputId)>> {
        let ref panner = self.panner;
        self.sources.get(id.0)
            .map(move |src| panner.generator(&src.inner_id).unwrap())
            .map(|gen| gen.input_source(0).unwrap())
    }

    /// Get a mutable reference to the source of the specified source.
    pub fn input_source_mut(
        &mut self,
        id: &SourceId,
    ) -> Option<&mut Option<(NodeId, OutputId)>> {
        let ref mut panner = self.panner;
        self.sources.get(id.0)
            .map(move |src| panner.generator_mut(&src.inner_id).unwrap())
            .map(|gen| gen.input_source_mut(0).unwrap())
    }

    /// Get the numboer of outputs.
    pub fn num_outputs(&self) -> usize {
        self.num_outputs
    }
}

impl<T> Node for PannerNode<T>
where
    T: Panner<NodeInputGenerator> + Debug + Sync + Send + 'static,
{
    fn num_outputs(&self) -> usize {
        self.num_outputs
    }

    fn inspect(&mut self, inspector: &mut NodeInspector) {
        for src in self.sources.iter() {
            let gen = self.panner.generator(&src.inner_id).unwrap();
            if let &Some(source) = gen.input_source(0).unwrap() {
                inspector.declare_input(source).finish();
            }
        }
    }

    fn render(&mut self, to: &mut [&mut [f32]], context: &NodeRenderContext) -> bool {
        let ref mut panner = self.panner;
        self.host.with(context, || {
            if panner.is_active() {
                let num_samples = to[0].len();
                panner.render(to, 0..num_samples);
                true
            } else {
                false
            }
        })
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}
