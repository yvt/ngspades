//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::any::Any;
use std::fmt::Debug;

use nodes::{NodeInspector, NodeRenderContext};

/// Audio processing node.
///
/// Every audio node has:
///
///  - A fixed number of **outputs**. Each output is identified by an `OutputId`
///    (alias of `usize`). The number of output, indicated by `num_outputs`,
///    must be determined in advance before the node is inserted to a `Context`
///    and must not change throughout its lifetime.
///
///  - An arbitrary (possibly dynamic) number of **inputs**. Users would use the
///    API provided by a node implementation to connect each input to another
///    node's output by specifying `(NodeId, OutputId)`.
///
///  - Other parameters may be exposed via a implementation-specfiic interface.
///    It may include time-varying values like `DynamicValue`, but keep in mind
///    that some node can operate in a different timeline.
///
pub trait Node: Any + Send + Sync + Debug {
    /// Get the number of outputs.
    ///
    /// Restriction due to the current implementation: it must be less than or
    /// equal to 64.
    fn num_outputs(&self) -> usize;

    /// Query the relationship with other nodes.
    ///
    /// The implementor must report the existence of every input by calling
    /// `NodeInspector::declare_input`.
    fn inspect(&mut self, scanner: &mut NodeInspector);

    /// Produce audio data.
    ///
    /// Returns `false` if it did not produce audio above a predetermined
    /// threshold. In this case, the implementation does not even have to fill
    /// the output buffer. For example, you could skip entire the computation
    /// and return `false` if all inputs were zero (i.e. `NodeInput::is_active()
    /// == false`).
    ///
    /// Returns `true` otherwise.
    fn render(&mut self, to: &mut [&mut [f32]], context: &NodeRenderContext) -> bool;

    /// Get the `Any` reference to `self`.
    ///
    /// This is a boilerplate method and you usually implement it as the following:
    ///
    /// ```ignore
    /// use std::any::Any;
    /// fn as_any(&self) -> &Any {
    ///     self
    /// }
    /// ```
    fn as_any(&self) -> &Any;

    /// Get the mutable `Any` reference to `self`.
    ///
    /// This is a boilerplate method and you usually implement it as the following:
    ///
    /// ```ignore
    /// use std::any::Any;
    /// fn as_any_mut(&mut self) -> &mut Any {
    ///     self
    /// }
    /// ```
    fn as_any_mut(&mut self) -> &mut Any;
}

/// Types that can be converted to `Box<Node>`, or are already `Box<Node>`.
///
/// Provided for ergonomic reason. See the following example:
///
///     use ysr2_common::nodes::{Node, ZeroNode, IntoNodeBox};
///
///     fn without_into_node_box(_: Box<Node>) {}
///     fn with_into_node_box<T: IntoNodeBox>(_: T) {}
///
///     without_into_node_box(Box::new(ZeroNode));
///
///     with_into_node_box(ZeroNode);
pub trait IntoNodeBox {
    fn into_box(self) -> Box<Node>;
}

impl IntoNodeBox for Box<Node> {
    fn into_box(self) -> Box<Node> {
        self
    }
}

impl<T: Node> IntoNodeBox for T {
    fn into_box(self) -> Box<Node> {
        Box::new(self)
    }
}
