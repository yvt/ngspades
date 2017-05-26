//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::hash::Hash;
use std::fmt::Debug;
use std::cmp::{Eq, PartialEq};
use std::any::Any;
use std::time::Duration;

use super::{Resources, PipelineStageFlags, DepthBias, DepthBounds, Viewport, Rect2D,
            Result, Framebuffer};

use enumflags::BitFlags;
use cgmath::Vector3;

pub trait CommandQueue<R: Resources, TCommandBuffer: CommandBuffer<R>>
    : Hash + Debug + Eq + PartialEq + Send + Any {
    fn make_command_buffer(&self) -> Result<TCommandBuffer>;

    /// Submit command buffers to a queue.
    ///
    /// If `fence` is specified, it will be signaled upon cmpletion of
    /// the execution. It must not be associated with any other
    /// commands that has not yet completed execution.
    fn submit_commands(&self,
                       submissions: &[&SubmissionInfo<R, TCommandBuffer>],
                       fence: Option<&R::Fence>)
                       -> Result<()>;

    fn wait_idle(&self);
}

#[derive(Debug, Copy, Clone)]
pub struct SubmissionInfo<'a, R: Resources, TCommandBuffer: CommandBuffer<R>> {
    pub buffers: &'a[&'a TCommandBuffer],
    pub wait_semaphores: &'a[&'a R::Semaphore],
    pub signal_semaphores: &'a[&'a R::Semaphore],
}

/// Command buffer.
///
/// When dropping a `CommandBuffer`, it must not be in the `Pending` state.
/// Also, it must not outlive the originating `CommandQueue`.
pub trait CommandBuffer<R: Resources>
    : Hash + Debug + Eq + PartialEq + Send + Any + CommandEncoder<R> {

    fn state(&self) -> CommandBufferState;
    fn wait_completion(&self, timeout: Duration) -> Result<bool>;
}


#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CommandBufferState {
    Initial,
    Recording,
    Executable,
    Pending,
    Error,
}

/// Encodes commands into a command buffer.
pub trait CommandEncoder<R: Resources>
    : Hash + Debug + Eq + PartialEq + Send + Any
{
    /// Start recording a command buffer.
    /// The existing contents will be cleared (if any).
    ///
    /// The command buffer must be in the `Initial`, `Executable`, or `Error` state.
    fn begin_encoding(&mut self);

    /// End recording a command buffer.
    ///
    /// The command buffer must be in the `Recording` state.
    fn end_encoding(&mut self);

    /// Begin a render pass.
    ///
    /// If a compute pipeline is currently bound, it will be unbound.
    fn begin_render_pass(&mut self, framebuffer: &R::Framebuffer);

    /// End a render pass.
    ///
    /// `next_subpass` must have been called enough times between
    /// calls to this and the matching `begin_render_pass`.
    ///
    /// If a graphics pipeline is currently bound, it will be unbound.
    fn end_render_pass(&mut self);

    /// Make a transition to the next subpass.
    /// Must be called for each subpass before `end_render_pass` is called.
    ///
    /// If a graphics pipeline is currently bound, it will be unbound.
    fn next_subpass(&mut self);

    /// Sets the current `GraphicsPipeline` object.
    ///
    /// A render pass must be active and compatible with the specified pipeline.
    ///
    /// All dynamic states will be reseted and all descriptors will be unbound.
    /// They all have to be specified before issuing the first draw call.
    fn bind_graphics_pipeline(&mut self, pipeline: &R::GraphicsPipeline);

    /// Specifies the dynamic blend constant values. The current `GraphicsPipeline`'s
    /// `blend_constants` must be `StaticOrDynamic::Dynamic`.
    fn set_blend_constants(&mut self, value: &[f32; 4]);

    /// Specifies the dynamic depth bias values. The current `GraphicsPipeline`'s
    /// `depth_bias` must be `StaticOrDynamic::Dynamic`.
    fn set_depth_bias(&mut self, value: &Option<DepthBias>);

    /// Specifies the dynamic depth bound values. The current `GraphicsPipeline`'s
    /// `depth_bounds` must be `StaticOrDynamic::Dynamic`.
    fn set_depth_bounds(&mut self, value: &Option<DepthBounds>);

    /// Sets the current `StencilState` object. The current `GraphicsPipeline`'s
    /// `stencil_state` must be `StaticOrDynamic::Dynamic`.
    fn set_stencil_state(&mut self, value: &R::StencilState);

    /// Specifies the dynamic viewport values. The current `GraphicsPipeline`'s
    /// `viewport` must be `StaticOrDynamic::Dynamic`.
    fn set_viewport(&mut self, value: &Viewport);

    /// Specifies the dynamic scissor rectangle. The current `GraphicsPipeline`'s
    /// `scissor_rect` must be `StaticOrDynamic::Dynamic`.
    fn set_scissor_rect(&mut self, value: &Rect2D<i32>);

    fn bind_descriptor_sets(&mut self,
                            pipeline_layout: &R::PipelineLayout,
                            start_index: usize,
                            descriptor_sets: &[R::DescriptorSet],
                            dynamic_offsets: &[u32]);

    fn draw(&mut self,
            num_vertices: u32,
            num_instances: u32,
            start_vertex_index: u32,
            start_instance_index: u32);

    fn draw_indexed(&mut self,
                    num_vertices: u32,
                    num_instances: u32,
                    start_vertex_index: u32,
                    index_offset: u32,
                    start_instance_index: u32);

    /// Set the current `ComputePipeline` object.
    ///
    /// Must not be called inside a render pass.
    fn bind_compute_pipeline(&mut self, pipeline: &R::ComputePipeline);

    /// Provoke work in a compute pipeline.
    ///
    /// There must be a bound `ComputePipeline`.
    fn dispatch(&mut self, workgroup_count: Vector3<u32>);

    // TODO: blit/copy/clear commands
}
