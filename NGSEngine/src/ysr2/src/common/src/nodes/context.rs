//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::collections::HashSet;
use atomic_refcell::{AtomicRefCell, AtomicRefMut};
use arrayvec::ArrayVec;
use nodes::Node;
use utils::{Pool, PoolPtr};

/// Encapsulates the audio node system's context.
///
/// See the documentation of [`Node`] for the concepts relevant to nodes.
///
/// [`Node`]: trait.Node.html
///
/// # Examples
///
/// The following example creates a context, and sets up some nodes in it.
///
///     # use ysr2_common::nodes::*;
///     let mut context = Context::new();
///
///     // Create a source
///     let source_id = context.insert(Box::new(ZeroNode));
///
///     // Create a sink and connect it to the source
///     let mut sink = OutputNode::new(1);
///     *sink.input_source_mut(0).unwrap() = Some((source_id, 0));
///     let sink_id = context.insert(Box::new(sink));
///
///     # (source_id, sink_id);
///
/// Now you can generate a few samples by requesting the `OutputNode` to pull
/// some inputs:
///
///     # use ysr2_common::nodes::*;
///     # let mut context = Context::new();
///     # let source_id = context.insert(Box::new(ZeroNode));
///     # let mut sink = OutputNode::new(1);
///     # *sink.input_source_mut(0).unwrap() = Some((source_id, 0));
///     # let sink_id = context.insert(Box::new(sink));
///     // Request a next frame
///     {
///         let sink = context.get_mut_as::<OutputNode>(&sink_id).unwrap();
///         sink.request_frame(512);
///     }
///
///     // Process the frame
///     context.render().unwrap();
///
///     // Read the output
///     {
///         let sink = context.get_as::<OutputNode>(&sink_id).unwrap();
///         let samples = sink.get_samples(0).unwrap();
///         assert_eq!(samples, &[0.0f32; 512][..]);
///     }
///
#[derive(Debug)]
pub struct Context {
    nodes: Pool<ContextNode>,
    sinks: HashSet<NodeId>,

    /// Indexed by `BufferId`
    buffers: Vec<AtomicRefCell<Buffer>>,
    sched_info: SchedInfo,
}

/// Indicates a problem encountered while processing nodes.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ContextError {
    /// One of the connections is invalid.
    InvalidConnection,

    /// One of the node has multiple consumers with different sample count
    /// requirements.
    SampleCountMismatch,

    /// A feedback loop was detected.
    FeedbackLoop,
}

/// Identifies a `Node` in a `Context`.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct NodeId(PoolPtr);

/// Identifies a output of a `Node`.
pub type OutputId = usize;

#[derive(Debug)]
struct ContextNode {
    node: Box<Node>,
}

#[derive(Debug, Clone)]
struct Buffer {
    data: Vec<f32>,
    state: BufferState,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            state: BufferState::InactiveDirty,
        }
    }
}

type BufferId = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
enum BufferState {
    Active,
    InactiveDirty,
    Inactive,
}

impl Context {
    /// Construct an empty `Context`.
    pub fn new() -> Self {
        Self {
            nodes: Pool::new(),
            sinks: HashSet::new(),
            buffers: Vec::new(),
            sched_info: SchedInfo::new(),
        }
    }

    /// Insert a node into the context.
    pub fn insert(&mut self, node: Box<Node>) -> NodeId {
        let num_outputs = node.num_outputs();

        let id = NodeId(self.nodes.allocate(ContextNode { node }));

        if num_outputs == 0 {
            assert!(self.sinks.insert(id));
        }

        let ref mut sched_info = self.sched_info;
        let index = (id.0).0;
        if index >= sched_info.node_sched_infos.len() {
            sched_info.node_sched_infos.resize(
                index + 1,
                Default::default(),
            );
        }
        sched_info.node_sched_infos[index] = NodeSchedInfo {
            num_output_samples: None,
            state: NodeState::Inactive,
            outputs: vec![ContextNodeOutput { buffer_index: None }; num_outputs],
        };

        id
    }

    /// Remove a node into the context.
    pub fn remove(&mut self, id: &NodeId) -> Option<Box<Node>> {
        self.sinks.remove(id);
        self.nodes.deallocate(id.0).map(|cn| cn.node)
    }

    /// Get a reference to a node in the context.
    pub fn get(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.get(id.0).map(|cn| &*cn.node)
    }

    /// Get a mutable reference to a node in the context.
    pub fn get_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id.0).map(|cn| &mut *cn.node)
    }

    /// Get a reference to a node of a given type in the context.
    ///
    /// Returns `None` if the node was not found or the concrete type did not
    /// match.
    pub fn get_as<T: Node>(&self, id: &NodeId) -> Option<&T> {
        self.get(id).and_then(|node| node.as_any().downcast_ref())
    }

    /// Get a mutable reference to a node of a given type in the context.
    ///
    /// Returns `None` if the node was not found or the concrete type did not
    /// match.
    pub fn get_mut_as<T: Node>(&mut self, id: &NodeId) -> Option<&mut T> {
        self.get_mut(id).and_then(
            |node| node.as_any_mut().downcast_mut(),
        )
    }

    pub fn render(&mut self) -> Result<(), ContextError> {
        let ref mut sched_info = self.sched_info;

        sched_info.schedule(
            &mut self.nodes,
            self.sinks.iter().map(Clone::clone),
        )?;

        // Allocate buffers as needed
        {
            let ref buffer_sched_infos = sched_info.buffer_sched_info.buffer_sched_infos;
            if buffer_sched_infos.len() > self.buffers.len() {
                self.buffers.resize(
                    buffer_sched_infos.len(),
                    Default::default(),
                );
            }

            for (bsi, buffer) in buffer_sched_infos.iter().zip(self.buffers.iter()) {
                // `AtomicRefCell` does not have `get_mut`, unfortunately
                let mut buffer = buffer.borrow_mut();
                if bsi.max_size > buffer.data.len() {
                    let extra = bsi.max_size - buffer.data.len();
                    buffer.data.reserve(extra);
                }
            }
        }

        // Execute each node in a scheduled order
        let ref buffers = self.buffers;
        for &node_id in sched_info.activated_nodes.iter().rev() {
            let ref nsi = sched_info.node_sched_infos[(node_id.0).0];
            let n_samples = nsi.num_output_samples;
            let mut out_refs: ArrayVec<[_; 64]> = nsi.outputs
                .iter()
                .map(|output| {
                    let mut buffer = buffers[output.buffer_index.unwrap()].borrow_mut();
                    buffer.data.resize(n_samples.unwrap(), 0.0);
                    buffer.state = BufferState::Active;
                    buffer
                })
                .collect();
            let active = {
                let mut outs: ArrayVec<[_; 64]> = out_refs
                    .iter_mut()
                    .map(|buffer| &mut buffer.data[..])
                    .collect();
                let ctx_node: &mut ContextNode = self.nodes.get_mut(node_id.0).unwrap();
                let context = NodeRenderContext {
                    node_sched_infos: &sched_info.node_sched_infos,
                    buffers: buffers,
                };
                ctx_node.node.render(&mut outs[..], &context)
            };
            for buffer in out_refs.iter_mut() {
                buffer.state = if active {
                    BufferState::Active
                } else {
                    BufferState::InactiveDirty
                };
            }
        }

        sched_info.cleanup();

        Ok(())
    }
}

#[derive(Debug)]
struct SchedInfo {
    activated_nodes: Vec<NodeId>,
    activation_stack: Vec<NodeId>,
    /// Indexed by `NodeId.0`
    node_sched_infos: Vec<NodeSchedInfo>,
    buffer_sched_info: BuffersSchedInfo,
}

#[derive(Debug, Clone)]
struct NodeSchedInfo {
    // The following fields are written during the preprocessing
    num_output_samples: Option<usize>,
    state: NodeState,
    outputs: Vec<ContextNodeOutput>,
}

impl Default for NodeSchedInfo {
    fn default() -> Self {
        NodeSchedInfo {
            num_output_samples: None,
            state: NodeState::Inactive,
            outputs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ContextNodeOutput {
    buffer_index: Option<BufferId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NodeState {
    Inactive,
    Found,
    Active,
}

impl SchedInfo {
    fn new() -> Self {
        Self {
            activated_nodes: Vec::new(),
            activation_stack: Vec::new(),
            node_sched_infos: Vec::new(),
            buffer_sched_info: BuffersSchedInfo::new(),
        }
    }

    fn schedule<S>(&mut self, nodes: &mut Pool<ContextNode>, sinks: S) -> Result<(), ContextError>
    where
        S: Iterator<Item = NodeId>,
    {
        assert!(self.activation_stack.is_empty());
        assert!(self.activated_nodes.is_empty());

        // We will traverse the graph in backward, with cycle detection
        for node_id in sinks {
            self.activation_stack.push(node_id);

            let ref mut nsi = self.node_sched_infos[(node_id.0).0];
            nsi.num_output_samples = None;
            nsi.state = NodeState::Found;
        }

        self.buffer_sched_info.reset();

        while let Some(node_id) = self.activation_stack.pop() {
            // Mark the node as visited
            self.activated_nodes.push(node_id);
            {
                let ref mut nsi = self.node_sched_infos[(node_id.0).0];
                debug_assert_eq!(nsi.state, NodeState::Found);
                nsi.state = NodeState::Active;
            }

            // Traverse via each edge in backward
            let ctx_node: &mut ContextNode = nodes.get_mut(node_id.0).unwrap();
            {
                let mut scanner = NodeInspector {
                    sched_info: self,
                    node_id,
                    error: None,
                };
                ctx_node.node.inspect(&mut scanner);
                if let Some(error) = scanner.error {
                    scanner.sched_info.cleanup();
                    return Err(error);
                }
            }

            {
                // Allocate scratch buffers
                // (buffers for these outputs are not allocated yet because no node
                //  actually uses them, but we still have to provide valid output
                //  buffers to the render method)
                let ref mut nsi: NodeSchedInfo = self.node_sched_infos[(node_id.0).0];
                if nsi.outputs.len() > 0 {
                    let n_samples = nsi.num_output_samples.unwrap();
                    for output in nsi.outputs.iter_mut() {
                        if output.buffer_index.is_none() {
                            output.buffer_index = Some(self.buffer_sched_info.allocate(n_samples));
                        }
                    }

                    // Release the output buffer
                    for output in nsi.outputs.iter_mut() {
                        self.buffer_sched_info.deallocate(
                            output.buffer_index.unwrap(),
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn cleanup(&mut self) {
        for node_id in self.activation_stack.drain(..) {
            let ref mut nsi = self.node_sched_infos[(node_id.0).0];
            nsi.state = NodeState::Inactive;
        }
        for node_id in self.activated_nodes.drain(..) {
            let ref mut nsi = self.node_sched_infos[(node_id.0).0];
            nsi.state = NodeState::Inactive;
        }
    }
}

/// Passed to `Node::inspect` to obtain the information about the relationship
/// with other nodes.
#[derive(Debug)]
pub struct NodeInspector<'a> {
    sched_info: &'a mut SchedInfo,
    node_id: NodeId,
    error: Option<ContextError>,
}

impl<'a> NodeInspector<'a> {
    /// Get the number of output samples for the next frame.
    ///
    /// Returns `None` for a sink node (one of those without any outputs).
    ///
    /// The returned value cannot be `Some(0)`.
    pub fn num_output_samples(&self) -> Option<usize> {
        self.sched_info
            .node_sched_infos
            .get((self.node_id.0).0)
            .unwrap()
            .num_output_samples
    }

    /// Start a declaration of a input of the current node.
    ///
    /// You must call `NodeInputDecl::finish()` after setting its properties to the
    /// desired values.
    pub fn declare_input<'b>(&'b mut self, source: (NodeId, OutputId)) -> NodeInputDecl<'a, 'b> {
        NodeInputDecl {
            num_samples: self.num_output_samples(),
            scanner: self,
            source,
        }
    }
}

/// Used with `NodeInspector` to declare a input of the current node.
pub struct NodeInputDecl<'a: 'b, 'b> {
    scanner: &'b mut NodeInspector<'a>,
    source: (NodeId, OutputId),
    num_samples: Option<usize>,
}

impl<'a: 'b, 'b> NodeInputDecl<'a, 'b> {
    /// Set the number of samples to consume.
    ///
    /// If `NodeInspector::num_output_samples()` is `Some(x)`, it defaults to `x`.
    /// Otherwise, it must be explicitly specified.
    ///
    /// `num_samples` must not be zero.
    pub fn num_samples(self, num_samples: usize) -> Self {
        assert_ne!(num_samples, 0);
        Self {
            num_samples: Some(num_samples),
            ..self
        }
    }

    /// Finish the declaration of the input.
    ///
    /// **Panics** if `num_samples` is not specified and no default value is
    /// provided (this is the case if the current node is a sink node, which
    /// does not have any outputs).
    pub fn finish(self) {
        let num_samples = self.num_samples.expect("num_samples is not specified");
        let ref mut sched_info = self.scanner.sched_info;

        // Find the source node
        let source_node = sched_info.node_sched_infos.get_mut(((self.source.0).0).0);
        let source_node: &mut NodeSchedInfo = if let Some(source_node) = source_node {
            source_node
        } else {
            self.scanner.error = Some(ContextError::InvalidConnection);
            return;
        };

        // Check cycle
        if source_node.state == NodeState::Active {
            self.scanner.error = Some(ContextError::FeedbackLoop);
            return;
        }

        // Reset the state if the node was found for the first time during this
        // frame
        if source_node.state == NodeState::Inactive {
            for x in source_node.outputs.iter_mut() {
                x.buffer_index = None;
            }
        }

        // Check and update the number of sample
        if let Some(num_output_samples) = source_node.num_output_samples {
            if num_output_samples != num_samples {
                self.scanner.error = Some(ContextError::SampleCountMismatch);
                return;
            }
        } else {
            source_node.num_output_samples = Some(num_samples);
        }

        // Find the source output
        let output: &mut ContextNodeOutput =
            if let Some(x) = source_node.outputs.get_mut(self.source.1) {
                x
            } else {
                self.scanner.error = Some(ContextError::InvalidConnection);
                return;
            };

        if output.buffer_index.is_none() {
            // Allocate the buffer for it
            output.buffer_index = Some(sched_info.buffer_sched_info.allocate(num_samples));
        }

        if source_node.state == NodeState::Inactive {
            // Traverse from this node later
            sched_info.activation_stack.push(self.source.0);
            source_node.state = NodeState::Found;
        }
    }
}

#[derive(Debug)]
struct BuffersSchedInfo {
    /// Indexed by `BufferId`
    ///
    /// FIXME: indexing by an internal of `PoolPtr` does not seem like a good idea
    buffer_sched_infos: Vec<BufferSchedInfo>,
    first_free: Option<BufferId>,
}

#[derive(Debug)]
struct BufferSchedInfo {
    max_size: usize,
    state: BufferSchedState,
}

#[derive(Debug, PartialEq, Eq)]
enum BufferSchedState {
    InUse,
    Free(Option<usize>),
}

impl BuffersSchedInfo {
    fn new() -> Self {
        BuffersSchedInfo {
            buffer_sched_infos: Vec::new(),
            first_free: None,
        }
    }

    fn reset(&mut self) {
        self.buffer_sched_infos.clear();
        self.first_free = None;
    }

    fn allocate(&mut self, size: usize) -> BufferId {
        if let Some(id) = self.first_free {
            let ref mut buffer = self.buffer_sched_infos[id];
            if size > buffer.max_size {
                buffer.max_size = size;
            }
            self.first_free = match buffer.state {
                BufferSchedState::InUse => unreachable!(),
                BufferSchedState::Free(next_free) => next_free,
            };
            buffer.state = BufferSchedState::InUse;
            id
        } else {
            let id: BufferId = self.buffer_sched_infos.len();
            self.buffer_sched_infos.push(BufferSchedInfo {
                max_size: size,
                state: BufferSchedState::InUse,
            });
            id
        }
    }

    fn deallocate(&mut self, id: BufferId) {
        debug_assert_eq!(self.buffer_sched_infos[id].state, BufferSchedState::InUse);
        self.buffer_sched_infos[id].state = BufferSchedState::Free(self.first_free);
        self.first_free = Some(id);
    }
}

/// Contextual information passed to `Node::render`.
#[derive(Debug)]
pub struct NodeRenderContext<'a> {
    node_sched_infos: &'a Vec<NodeSchedInfo>,
    buffers: &'a Vec<AtomicRefCell<Buffer>>,
}

/// Node input information returned by `NodeRenderContext::get_input`.
#[derive(Debug)]
pub struct NodeInput<'a> {
    buffer: AtomicRefMut<'a, Buffer>,
}

impl<'a> NodeRenderContext<'a> {
    /// Get a input signal.
    ///
    /// Might return `None` if `target` is unknown.
    pub fn get_input(&self, target: (NodeId, OutputId)) -> Option<NodeInput> {
        self.node_sched_infos
            .get(((target.0).0).0)
            .and_then(|nsi| nsi.outputs.get(target.1))
            .and_then(|cno| cno.buffer_index)
            .map(|index| {
                NodeInput { buffer: self.buffers[index].borrow_mut() }
            })
    }
}

impl<'a> NodeInput<'a> {
    /// Get the slice of the input signal.
    ///
    /// Note that if the input was inactive, calling this method would trigger
    /// a zero-fill operation. You can avoid this by checking if it is inactive
    /// by calling `is_active()` beforehand.
    pub fn samples(&mut self) -> &[f32] {
        if self.buffer.state == BufferState::InactiveDirty {
            for x in self.buffer.data.iter_mut() {
                *x = 0.0;
            }
            self.buffer.state = BufferState::Inactive;
        }
        self.buffer.data.as_slice()
    }

    /// Check if the input is active or not (or in other words, `samples()` has
    /// at least one significant sample).
    pub fn is_active(&mut self) -> bool {
        self.buffer.state == BufferState::Active
    }
}
