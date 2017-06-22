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

use {Backend, PipelineStageFlags, DepthBias, DepthBounds, Viewport, Rect2D, Result, Framebuffer,
     Marker, ImageSubresourceRange, IndexFormat, ImageLayout, AccessFlags};

use enumflags::BitFlags;
use cgmath::Vector3;

pub trait CommandQueue<B: Backend>: Debug + Send + Any + Marker {
    fn make_command_buffer(&self) -> Result<B::CommandBuffer>;

    /// Submit command buffers to a queue.
    ///
    /// The specified command buffers must be in the `Executable` state.
    ///
    /// If `fence` is specified, it will be signaled upon cmpletion of
    /// the execution. It must not be associated with any other
    /// commands that has not yet completed execution.
    fn submit_commands(
        &self,
        submissions: &[&SubmissionInfo<B>],
        fence: Option<&B::Fence>,
    ) -> Result<()>;

    fn wait_idle(&self);
}

#[derive(Debug, Copy, Clone)]
pub struct SubmissionInfo<'a, B: Backend> {
    pub buffers: &'a [&'a B::CommandBuffer],
    pub wait_semaphores: &'a [&'a B::Semaphore],
    pub signal_semaphores: &'a [&'a B::Semaphore],
}

/// Command buffer.
///
/// When dropping a `CommandBuffer`, it must not be in the `Pending` state.
/// Also, it must not outlive the originating `CommandQueue`.
pub trait CommandBuffer<B: Backend>
    : Debug + Send + Any + CommandEncoder<B> + Marker {
    fn state(&self) -> CommandBufferState;

    /// Stall the current threa until the execution of the command buffer
    /// completes.
    ///
    /// The current state must be one of `Pending` and `Completed`.
    fn wait_completion(&self, timeout: Duration) -> Result<bool>;
}


pub trait SecondaryCommandBuffer<B: Backend>
    : Debug + Send + Any + RenderSubpassCommandEncoder<B> + Marker {
    /// End recording a second command buffer.
    fn end_encoding(&mut self);
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CommandBufferState {
    Initial,
    Recording,
    Executable,
    Pending,
    Completed,
    Error,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum RenderPassContents {
    Inline,
    SecondaryCommandBuffers,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SubresourceWithLayout<'a, B: Backend> {
    Image {
        image: &'a B::Image,
        range: ImageSubresourceRange,
        layout: ImageLayout,
    },
    Buffer {
        buffer: &'a B::Buffer,
        offset: usize,
        len: usize,
    },
}

/// Describes a memory barrier.
///
/// Please see Vulkan 1.0 Specification "6.7. Memory Barriers".
///
/// TODO: add `#[derive(Hash)]` after `enumflags` was updated to
///       implement that on `BitFlags`
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Barrier<'a, B: Backend> {
    AcquireQueueOwnership {
        resource: SubresourceWithLayout<'a, B>,
        // TODO: make this point to a queue family, not `CommandQueue` (which is not `Sync`)
        source: &'a B::CommandQueue,
    },
    ReleaseQueueOwnership {
        resource: SubresourceWithLayout<'a, B>,
        // TODO: make this point to a queue family, not `CommandQueue` (which is not `Sync`)
        destination: &'a B::CommandQueue,
    },
    GlobalMemoryBarrier {
        source_access_mask: BitFlags<AccessFlags>,
        destination_access_mask: BitFlags<AccessFlags>,
    },
    ImageMemoryBarrier {
        image: &'a B::Image,
        source_access_mask: BitFlags<AccessFlags>,
        destination_access_mask: BitFlags<AccessFlags>,
        source_layout: ImageLayout,
        destination_layout: ImageLayout,
    },
    BufferMemoryBarrier {
        buffer: &'a B::Buffer,
        source_access_mask: BitFlags<AccessFlags>,
        destination_access_mask: BitFlags<AccessFlags>,
        offset: usize,
        len: usize,
    },
}

/// Encodes commands into a command buffer.
pub trait CommandEncoder<B: Backend>
    : Debug
    + Send
    + Any
    + RenderSubpassCommandEncoder<B>
    + ComputeCommandEncoder<B>
    + BlitCommandEncoder<B> {
    /// Start recording a command buffer.
    /// The existing contents will be cleared (if any).
    ///
    /// The command buffer must be in the `Initial`, `Executable`, `Completed`, or `Error` state.
    fn begin_encoding(&mut self);

    /// End recording a command buffer.
    ///
    /// The command buffer must be in the `Recording` state.
    /// No kind of passes can be active at the point of call.
    fn end_encoding(&mut self);

    /// Inserts a memory dependency (which also implies an execution dependency)
    /// between commands that were submitted before this and those submitted
    /// after it.
    ///
    /// There must not be an active render/compute/blit pass.
    fn barrier(
        &mut self,
        source_stage: BitFlags<PipelineStageFlags>,
        destination_stage: BitFlags<PipelineStageFlags>,
        barriers: &[Barrier<B>],
    );

    /// Begin a render pass.
    ///
    /// During a render pass, calls to `begin_render_subpass` and `end_render_subpass`
    /// must occur as many times as the number of subpasses in the render pass associated
    /// with the specified framebuffer.
    fn begin_render_pass(&mut self, framebuffer: &B::Framebuffer);

    /// Begin a compute pass.
    ///
    /// Only during a compute pass, functions from `ComputeCommandEncoder` can be called.
    fn begin_compute_pass(&mut self);

    /// Begin a blit pass.
    ///
    /// Only during a compute pass, functions from `BlitCommandEncoder` can be called.
    fn begin_blit_pass(&mut self);

    /// Creates a secondary command buffer to encode commands from multiple threads.
    ///
    /// A render pass must be active with `RenderPassContents::SecondaryCommandBuffers`.
    /// `end_encoding` of the returned secondary buffer must be called before the
    /// current render subpass is completed by `next_render_subpass`.
    /// The application must perform adequate inter-thread synchronizations.
    fn make_secondary_command_buffer(&mut self) -> B::SecondaryCommandBuffer;

    /// End the current render, compute, or blit pass.
    ///
    /// If the current pass is a render pass,
    /// `begin_render_subpass` must have been called enough times since the last time
    /// `begin_render_pass` was called on this command encoder.
    fn end_pass(&mut self);

    /// Begin the next subpass.
    /// Must be called for each subpass before `end_pass` is called.
    ///
    /// `end_render_subpass` must be called after all commands for the current subpass
    /// were encoded.
    ///
    /// `contents` specifies the method how the contents of the render pass is
    /// encoded.
    ///
    /// - If `Inline` is specified, the contents are encoded by calling
    ///   functions from `RenderSubpassCommandEncoder` on `self`.
    /// - If `SecondaryCommandBuffers` is specified, the contents are encoded
    ///   via secondary command buffers, which are created by calling
    ///   `make_secondary_command_buffer` on `self`. Before proceeding to the
    ///   next subpass, all secondary command buffers have their encoding completed
    ///   via `end_encoding`.
    ///
    /// Only during a render subpass, functions from `RenderSubpassCommandEncoder` can be called.
    fn begin_render_subpass(&mut self, contents: RenderPassContents);

    /// End the current subpass.
    ///
    /// A render subpass must be active.
    fn end_render_subpass(&mut self);
}

/// Encodes render commands into a command buffer.
pub trait RenderSubpassCommandEncoder<B: Backend>: Debug + Send + Any {
    /// Sets the current `GraphicsPipeline` object.
    ///
    /// A render pass must be active and compatible with the specified pipeline.
    ///
    /// All dynamic states will be reseted. Descriptor sets with incompatible
    /// layouts (see Vulkan 1.0 Specification "13.2.2. Pipeline Layouts") will be
    /// unbound.
    fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline);

    /// Specifies the dynamic blend constant values. The current `GraphicsPipeline`'s
    /// `blend_constants` must be `StaticOrDynamic::Dynamic`.
    fn set_blend_constants(&mut self, value: &[f32; 4]);

    /// Specifies the dynamic depth bias values. The current `GraphicsPipeline`'s
    /// `depth_bias` must be `StaticOrDynamic::Dynamic`.
    fn set_depth_bias(&mut self, value: Option<DepthBias>);

    /// Specifies the dynamic depth bound values. The current `GraphicsPipeline`'s
    /// `depth_bounds` must be `StaticOrDynamic::Dynamic`.
    fn set_depth_bounds(&mut self, value: Option<DepthBounds>);

    /// Sets the current `StencilState` object. The current `GraphicsPipeline`'s
    /// `stencil_state` must be `StaticOrDynamic::Dynamic`.
    fn set_stencil_state(&mut self, value: &B::StencilState);

    /// Specifies the dynamic viewport values. The current `GraphicsPipeline`'s
    /// `viewport` must be `StaticOrDynamic::Dynamic`.
    fn set_viewport(&mut self, value: &Viewport);

    /// Specifies the dynamic scissor rectangle. The current `GraphicsPipeline`'s
    /// `scissor_rect` must be `StaticOrDynamic::Dynamic`.
    fn set_scissor_rect(&mut self, value: &Rect2D<u32>);

    fn bind_descriptor_sets(
        &mut self,
        pipeline_layout: &B::PipelineLayout,
        start_index: usize,
        descriptor_sets: &[B::DescriptorSet],
        dynamic_offsets: &[u32],
    );

    fn bind_vertex_buffers(&mut self, start_index: usize, buffers: &[(&B::Buffer, usize)]);

    fn bind_index_buffer(&mut self, buffer: &B::Buffer, offset: usize, format: IndexFormat);

    fn draw(
        &mut self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        start_instance_index: u32,
    );

    fn draw_indexed(
        &mut self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        index_offset: u32,
        start_instance_index: u32,
    );

    // TODO: indirect draw
}

/// Encodes compute commands into a command buffer.
pub trait ComputeCommandEncoder<B: Backend>: Debug + Send + Any {
    /// Set the current `ComputePipeline` object.
    ///
    /// A compute pass must be active.
    fn bind_compute_pipeline(&mut self, pipeline: &B::ComputePipeline);

    /// Provoke work in a compute pipeline.
    ///
    /// A compute pass must be active and a compute pipeline must be bound.
    fn dispatch(&mut self, workgroup_count: Vector3<u32>);
}

/// Encodes blit commands into a command buffer.
pub trait BlitCommandEncoder<B: Backend>: Debug + Send + Any {
    // TODO
}
