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
            Result, RenderPass, Framebuffer};

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
                       buffers: &[&TCommandBuffer],
                       fence: Option<&R::Fence>)
                       -> Result<()>;

    fn wait_idle(&self);
}

/// Command buffer.
///
/// When dropping a `CommandBuffer`, it must not be in the `Pending` state.
/// Also, it must not outlive the originating `CommandQueue`.
pub trait CommandBuffer<R: Resources>
    : Hash + Debug + Eq + PartialEq + Send + Any {
    type GraphicsCommandEncoder: GraphicsCommandEncoder<R>;
    type ComputeCommandEncoder: ComputeCommandEncoder<R>;
    type BlitCommandEncoder: BlitCommandEncoder<R>;

    /// Clear the contents of the command buffer.
    fn reset(&mut self);

    fn state(&self) -> CommandBufferState;
    fn wait_completion(&self, timeout: Duration) -> Result<bool>;

    fn graphics_command_encoder(&mut self,
                                description: &GraphicsCommandEncoderDescription<R::Framebuffer>)
                                -> &mut Self::GraphicsCommandEncoder;
    fn compute_command_encoder(&mut self) -> &mut Self::ComputeCommandEncoder;
    fn blit_command_encoder(&mut self) -> &mut Self::BlitCommandEncoder;
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicsCommandEncoderDescription<'a, TFramebuffer: Framebuffer> {
    /// Specifies the framebuffer to render onto.
    framebuffer: &'a TFramebuffer,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CommandBufferState {
    Initial,
    Recording,
    Executable,
    Pending,
    Invalid,
}

pub trait CommandEncoder<R: Resources>
    : Hash + Debug + Eq + PartialEq + Send + Any {
    /// Pushes a command that instructs a device to wait on the specified semaphore.
    ///
    /// You cannot wait and signal on the same semaphore in a single command encoder because
    /// some backends and drivers might wait for semaphores at the beginning of the
    /// command encoder.
    fn wait_semaphore(&mut self,
                      semaphore: &R::Semaphore,
                      stage_mask: BitFlags<PipelineStageFlags>);

    /// Pushes a command that instructs a device to signal the specified semaphore.
    ///
    /// You cannot wait and signal on the same semaphore in a single command encoder because
    /// some backends and drivers might signal semaphores at the end of the command encoder.
    fn signal_semaphore(&mut self, semaphore: &R::Semaphore);

    fn end_encoding(&mut self);
}

pub trait GraphicsCommandEncoder<R: Resources>
    : Hash + Debug + Eq + PartialEq + Send + Any + CommandEncoder<R> {
    /// Make a transition to the next subpass.
    /// Must be called for each subpass before `end_encoding` is called.
    fn next_subpass(&mut self);

    /// Sets the current `GraphicsPipeline` object.
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
}

pub trait ComputeCommandEncoder<R: Resources>
    : Hash + Debug + Eq + PartialEq + Send + Any + CommandEncoder<R> {
    /// Sets the current `ComputePipeline` object.
    fn bind_compute_pipeline(&mut self, pipeline: &R::ComputePipeline);

    fn dispatch(&mut self, workgroup_count: Vector3<u32>);
}

pub trait BlitCommandEncoder<R: Resources>
    : Hash + Debug + Eq + PartialEq + Send + Any + CommandEncoder<R> {
    // TODO
}

