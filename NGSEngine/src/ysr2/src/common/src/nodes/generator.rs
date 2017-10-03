//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{ptr, mem};
use std::sync::Arc;
use std::ops::Range;
use parking_lot::{RwLock, RwLockReadGuard};
use nodes::{NodeRenderContext, NodeId, OutputId};
use stream::Generator;

/// Manages `NodeInputGenerator`s.
#[derive(Debug)]
pub struct NodeInputGeneratorHost {
    state: Arc<RwLock<NodeInputGeneratorState>>,
}

#[derive(Debug)]
struct NodeInputGeneratorState {
    context: *const NodeRenderContext<'static>,
    current_frame: u64,
}

unsafe impl Send for NodeInputGeneratorState {}
unsafe impl Sync for NodeInputGeneratorState {}

/// Converts a node input to `Generator`.
///
/// `NodeInputGenerator` is created by `NodeInputGeneratorHost::make_generator`
/// and its instance can operate only during the duration of a call to
/// `NodeInputGeneratorHost::with`.
#[derive(Debug)]
pub struct NodeInputGenerator {
    state: StateRef,
    position: GeneratorPosition,
    inputs: Vec<Option<(NodeId, OutputId)>>,
}

#[derive(Debug)]
struct GeneratorPosition {
    current_frame: u64,
    position: usize,
}

impl NodeInputGeneratorHost {
    /// Construct a `NodeInputGeneratorHost`.
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(NodeInputGeneratorState {
                context: ptr::null(),
                current_frame: 1,
            })),
        }
    }

    /// Create a `NodeInputGenerator`.
    ///
    /// The returned `NodeInputGenerator` only can operate within the duration
    /// of a call to `with`.
    pub fn make_generator(&self, num_channels: usize) -> NodeInputGenerator {
        NodeInputGenerator {
            state: StateRef(Clone::clone(&self.state)),
            position: GeneratorPosition {
                current_frame: 0,
                position: 0,
            },
            inputs: vec![None; num_channels],
        }
    }

    /// Activate `NodeInputGenerator`s throughout the duration of the given
    /// function being called.
    ///
    /// When a `NodeInputGenerator` was requested to produce a output, it will
    /// read samples from a `NodeInput` obtained from the `NodeRenderContext`.
    /// Every call to `with` resets the read position to `0`.
    pub fn with<F: FnOnce() -> R, R>(&mut self, context: &NodeRenderContext, f: F) -> R {
        use std::mem::drop;

        let activator = Activator::new(self, context);
        let returned_value = f();
        drop(activator);

        returned_value
    }
}

struct Activator<'a>(&'a mut NodeInputGeneratorHost);

impl<'a> Activator<'a> {
    pub fn new(parent: &'a mut NodeInputGeneratorHost, context: &NodeRenderContext) -> Self {
        {
            let mut state = parent.state.write();
            state.current_frame = state.current_frame.checked_add(1).unwrap();
            state.context = unsafe { mem::transmute(context) };
        }

        Activator(parent)
    }
}

impl<'a> Drop for Activator<'a> {
    fn drop(&mut self) {
        let mut state = self.0.state.write();
        state.context = ptr::null();
    }
}

#[derive(Debug)]
struct StateRef(Arc<RwLock<NodeInputGeneratorState>>);

impl StateRef {
    fn context(&self) -> ContextAccessor {
        let state = self.0.read();
        if state.context.is_null() {
            panic!("NodeInputGenerator is not active (used outside a call to `with`)");
        }

        ContextAccessor(state)
    }
}

impl GeneratorPosition {
    fn update_frame(&mut self, ca: &ContextAccessor) {
        if self.current_frame != ca.0.current_frame {
            self.position = 0;
            self.current_frame = ca.0.current_frame;
        }
    }
}

struct ContextAccessor<'a>(RwLockReadGuard<'a, NodeInputGeneratorState>);

impl NodeInputGenerator {
    /// Get a reference to the value specifying the source of the specified
    /// output channel.
    pub fn input_source(&self, output_index: usize) -> Option<&Option<(NodeId, OutputId)>> {
        self.inputs.get(output_index)
    }

    /// Get a mutable reference to the value specifying the source of the
    /// specified output channel.
    pub fn input_source_mut(
        &mut self,
        output_index: usize,
    ) -> Option<&mut Option<(NodeId, OutputId)>> {
        self.inputs.get_mut(output_index)
    }
}

impl<'a> ::std::ops::Deref for ContextAccessor<'a> {
    type Target = NodeRenderContext<'a>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.context }
    }
}

impl Generator for NodeInputGenerator {
    fn render(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        // Fail fast
        for ch in to.iter() {
            &ch[range.clone()];
        }
        assert_eq!(to.len(), self.inputs.len(), "channel count mismatch");

        let context = self.state.context();
        let ref mut pos = self.position;
        pos.update_frame(&context);

        let read_range = pos.position..pos.position + range.len();
        for (ch, input) in to.iter_mut().zip(self.inputs.iter()) {
            if let &Some(source) = input {
                let mut node_input = context.get_input(source).unwrap();
                let samples = node_input.samples();
                ch[range.clone()].copy_from_slice(&samples[read_range.clone()]);
            } else {
                for x in ch.iter_mut() {
                    *x = 0.0;
                }
            }
        }

        pos.position += range.len();
    }

    fn skip(&mut self, num_samples: usize) {
        let context = self.state.context();
        let ref mut pos = self.position;
        pos.update_frame(&context);
        pos.position += num_samples;
    }

    fn is_active(&self) -> bool {
        let context = self.state.context();
        for input in self.inputs.iter() {
            if let &Some(source) = input {
                if context.get_input(source).unwrap().is_active() {
                    return true;
                }
            }
        }
        false
    }
}
