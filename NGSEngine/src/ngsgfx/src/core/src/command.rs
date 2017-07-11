//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Command queues and command buffers.
//!
//! TODO: provide a documentation on command passes and device engines
use std::fmt::Debug;
use std::any::Any;
use std::ops::Range;

use enumflags::BitFlags;

use {Backend, PipelineStageFlags, DepthBias, DepthBounds, Viewport, Rect2D, Result, Marker,
     ImageSubresourceRange, IndexFormat, ImageLayout, AccessTypeFlags, DebugMarker,
     FenceDescription, DescriptorSetBindingLocation, DeviceSize, VertexBindingLocation};

use cgmath::Vector3;

pub trait CommandQueue<B: Backend>: Debug + Send + Any + Marker {
    fn make_command_buffer(&self) -> Result<B::CommandBuffer>;
    fn make_fence(&self, description: &FenceDescription) -> Result<B::Fence>;

    /// Submit command buffers to a queue.
    ///
    /// The specified command buffers must be in the `Executable` state.
    ///
    /// If `event` is specified, it will be signaled upon cmpletion of
    /// the execution. It must not be associated with any other
    /// commands that has not yet completed execution.
    fn submit_commands(
        &self,
        buffers: &[&B::CommandBuffer],
        event: Option<&B::Event>,
    ) -> Result<()>;

    fn wait_idle(&self);
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
    fn wait_completion(&self) -> Result<()>;
}


pub trait SecondaryCommandBuffer<B: Backend>
    : Debug + Send + Any + RenderSubpassCommandEncoder<B> + Marker + BarrierCommandEncoder<B>
    {
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
        offset: DeviceSize,
        len: DeviceSize,
    },
}

/// Encodes commands into a command buffer.
///
/// Command Passes
/// --------------
///
/// Most commands only can be submitted during a command pass.
/// There are three types of command passes:
///
///  - Render pass, only during which render commands defined in [`RenderSubpassCommandEncoder`]
///    can be encoded. A render pass can be started with [`begin_render_pass`].
///  - Compute pass, only during which compute commands defined in [`ComputeCommandEncoder`]
///    can be encoded. A compute pass can be started with [`begin_compute_pass`].
///  - Copy pass, only during which copy commands defined in [`CopyCommandEncoder`]
///    can be encoded. A copy pass can be started with [`begin_copy_pass`].
///
/// After it was started, a command pass is said to be *active* until it is
/// ended by a call to [`end_pass`]. Only one command pass can be active at
/// the same time, so you must ensure to call `end_pass` before starting a new
/// one.
///
/// Furthermore, a render pass can contain one or more subpasses. During a
/// render pass, [`begin_render_subpass`] and [`end_render_subpass`]
/// must be called for every subpass specified in [`RenderPassDescription`]
/// used to create the [`RenderPass`] associated with the specified
/// [`Framebuffer`].
///
/// [`RenderSubpassCommandEncoder`]: trait.RenderSubpassCommandEncoder.html
/// [`ComputeCommandEncoder`]: trait.ComputeCommandEncoder.html
/// [`CopyCommandEncoder`]: trait.CopyCommandEncoder.html
/// [`begin_render_pass`]: #tymethod.begin_render_pass
/// [`begin_compute_pass`]: #tymethod.begin_compute_pass
/// [`begin_copy_pass`]: #tymethod.begin_copy_pass
/// [`end_pass`]: #tymethod.end_pass
/// [`begin_render_subpass`]: #tymethod.begin_render_subpass
/// [`end_render_subpass`]: #tymethod.end_render_subpass
/// [`RenderPassDescription`]: ../renderpass/struct.RenderPassDescription.html
/// [`RenderPass`]: ../renderpass/trait.RenderPass.html
/// [`Framebuffer`]: ../framebuffer/trait.Framebuffer.html
///
/// Engine
/// ------
///
/// Device engines ([`DeviceEngine`]) represent different parts of the hardware
/// that can process commands concurrently.
///
/// Every pass is associated with one of device engine other than `Host`.
///
/// Every subresource can be used by only one device engine at the same time.
/// Also, you need to perform a *engine ownership transfer operation* before
/// using a subresource in a engine other than the engine which was previously
/// accessing the subresource. The engine ownership transfer operation can be
/// performed by a call to [`release_resource`] in the source engine followed by
/// another call to [`acquire_resource`] in the destination engine.
/// You must make sure `acquire_resource` happens-after `release_resource` by
/// using appropriate synchronization primitives (e.g., `Fence` or
/// `CommandBuffer::wait_completion`).
/// If the source or destination engine is `Host` then the corresponding call to
/// `release_resource` or `acquire_resource` (respectively) is not required
/// (in fact, it is impossible since there is no way to start a command pass with
/// the `Host` engine).
///
/// [`DeviceEngine`]: enum.DeviceEngine.html
/// [`release_resource`]: #tymethod.release_resource
/// [`acquire_resource`]: #tymethod.acquire_resource
///
pub trait CommandEncoder<B: Backend>
    : Debug
    + Send
    + Any
    + RenderSubpassCommandEncoder<B>
    + ComputeCommandEncoder<B>
    + CopyCommandEncoder<B>
    + BarrierCommandEncoder<B> {
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

    /// Acquire an ownership of the specified resource.
    ///
    /// There must be an active pass of any type.
    /// During a render pass, this must be called before the first subpass is started.
    fn acquire_resource(
        &mut self,
        stage: PipelineStageFlags,
        access: AccessTypeFlags,
        from_engine: DeviceEngine,
        resource: &SubresourceWithLayout<B>,
    );

    /// Release an ownership of the specified resource.
    ///
    /// There must be an active pass of any type.
    /// During a render pass, this must be called after the last subpass was ended.
    fn release_resource(
        &mut self,
        stage: PipelineStageFlags,
        access: AccessTypeFlags,
        to_engine: DeviceEngine,
        resource: &SubresourceWithLayout<B>,
    );

    /// Begin a render pass.
    ///
    /// During a render pass, calls to `begin_render_subpass` and `end_render_subpass`
    /// must occur as many times as the number of subpasses in the render pass associated
    /// with the specified framebuffer.
    ///
    /// `engine` must be `Universal`.
    fn begin_render_pass(&mut self, framebuffer: &B::Framebuffer, engine: DeviceEngine);

    /// Begin a compute pass.
    ///
    /// Only during a compute pass, functions from `ComputeCommandEncoder` can be called.
    ///
    /// `engine` must not be `Copy` nor `Host`.
    fn begin_compute_pass(&mut self, engine: DeviceEngine);

    /// Begin a copy pass.
    ///
    /// Only during a compute pass, functions from `CopyCommandEncoder` can be called.
    /// `engine` must not be `Host`.
    fn begin_copy_pass(&mut self, engine: DeviceEngine);

    /// Creates a secondary command buffer to encode commands from multiple threads.
    ///
    /// A render pass must be active with `RenderPassContents::SecondaryCommandBuffers`.
    /// `end_encoding` of the returned secondary buffer must be called before the
    /// current render subpass is completed by `next_render_subpass`.
    /// The application must perform adequate inter-thread synchronizations.
    fn make_secondary_command_buffer(&mut self) -> B::SecondaryCommandBuffer;

    /// End the current render, compute, or copy pass.
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

/// Encodes barrier commands into a command buffer.
pub trait BarrierCommandEncoder<B: Backend>
    : Debug + Send + Any + DebugCommandEncoder {
    /// Instruct the device to wait until the given fence is reached.
    /// There must be an active compute/copy pass or render subpass.
    ///
    /// The backend might move the fence wait operation to the beginning of the
    /// pass or subpass (if the current pass is a render pass).
    ///
    /// You cannot wait on a fence that was updated in the same render pass as
    /// one that is currently active. Use subpass dependencies instead.
    ///
    /// `stage` and `access` specify pipeline stages and memory access types,
    /// respectively, that must wait until the given fence is reached.
    fn wait_fence(&mut self, stage: PipelineStageFlags, access: AccessTypeFlags, fence: &B::Fence);

    /// Instruct the device to update the given fence.
    /// There must be an active compute/copy pass or render subpass.
    ///
    /// The backend might delay the fence update operation until the end of the
    /// pass or subpass (if the current pass is a render pass).
    ///
    /// `stage` and `access` specify pipeline stages and memory access types,
    /// respectively, that must be completed before the fence is updated.
    fn update_fence(
        &mut self,
        stage: PipelineStageFlags,
        access: AccessTypeFlags,
        fence: &B::Fence,
    );

    /// Insert a resource barrier.
    ///
    /// There must be an active pass of any type.
    /// During a render pass, there must be an active subpass.
    ///
    /// FIXME: when is this required in place of `Fence`?
    fn resource_barrier(
        &mut self,
        source_stage: PipelineStageFlags,
        source_access: AccessTypeFlags,
        destination_stage: PipelineStageFlags,
        destination_access: AccessTypeFlags,
        resource: &SubresourceWithLayout<B>,
    );

    /// Instruct the device to convert the image layout of the given image into another one.
    ///
    /// There must be an active pass of any type.
    /// During a render pass, there must be an active subpsas.
    fn image_layout_transition(
        &mut self,
        source_stage: PipelineStageFlags,
        source_access: AccessTypeFlags,
        source_layout: ImageLayout,
        destination_stage: PipelineStageFlags,
        destination_layout: ImageLayout,
        destination_access: AccessTypeFlags,
        image: &B::Image,
    );
}

/// Encodes render commands into a command buffer.
pub trait RenderSubpassCommandEncoder<B: Backend>
    : Debug + Send + Any + DebugCommandEncoder {
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

    fn bind_graphics_descriptor_sets(
        &mut self,
        pipeline_layout: &B::PipelineLayout,
        start_index: DescriptorSetBindingLocation,
        descriptor_sets: &[&B::DescriptorSet],
        dynamic_offsets: &[u32],
    );

    fn bind_vertex_buffers(
        &mut self,
        start_index: VertexBindingLocation,
        buffers: &[(&B::Buffer, DeviceSize)],
    );

    fn bind_index_buffer(&mut self, buffer: &B::Buffer, offset: DeviceSize, format: IndexFormat);

    /// Renders primitives.
    ///
    /// `vertex_range` specifies the consecutive range of vertex indices to draw.
    ///
    /// The primivies are drawn for `instance_range.len()` times.
    /// Specify `0..1` to perform a normal (not instanced) rendering.
    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>);

    /// Renders primitives using a currently bound index buffer.
    ///
    /// Vertex indices are retrived from the consecutive range of index buffer
    /// specified by `index_buffer_range`.
    /// Before indexing into the vertex buffers, the value of `vertex_offset` is
    /// added to the vertex index.
    ///
    /// The primivies are drawn for `instance_range.len()` times. Specify `0..1`
    /// for `instance_range` to perform a normal (not instanced) rendering.
    ///
    /// The largest index value (`0xffff` for `U16` or `0xffffffff` for `U32`)
    /// is used for primitive restart functionality.
    /// This functionality is unavailable to "list" primitive topologies.
    /// For such topologies, the largest index value simply should not be used
    /// (due to compatibility issues).
    fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    );

    // TODO: indirect draw
}

/// Encodes compute commands into a command buffer.
pub trait ComputeCommandEncoder<B: Backend>
    : Debug + Send + Any + DebugCommandEncoder {
    /// Set the current `ComputePipeline` object.
    ///
    /// A compute pass must be active.
    fn bind_compute_pipeline(&mut self, pipeline: &B::ComputePipeline);

    fn bind_compute_descriptor_sets(
        &mut self,
        pipeline_layout: &B::PipelineLayout,
        start_index: DescriptorSetBindingLocation,
        descriptor_sets: &[&B::DescriptorSet],
        dynamic_offsets: &[u32],
    );

    /// Provoke work in a compute pipeline.
    ///
    /// A compute pass must be active and a compute pipeline must be bound.
    fn dispatch(&mut self, workgroup_count: Vector3<u32>);
}

/// Encodes copy commands into a command buffer.
pub trait CopyCommandEncoder<B: Backend>
    : Debug + Send + Any + DebugCommandEncoder {
    /// Copy data from a buffer to another buffer.
    fn copy_buffer(
        &mut self,
        source: &B::Buffer,
        source_offset: DeviceSize,
        destination: &B::Buffer,
        destination_offset: DeviceSize,
        size: DeviceSize,
    );

    // TODO: more commands
}

/// Encodes debug markers into a command buffer.
///
/// FIXME: can we allow these functions to be called inside a render pass with
/// `contents == SecondaryCommandBuffer`?
pub trait DebugCommandEncoder: Debug + Send + Any {
    /// Begin a debug group.
    ///
    /// A graphics subpass or a copy/compute pass must be active.
    fn begin_debug_group(&mut self, marker: &DebugMarker);

    /// End a debug group.
    ///
    /// There must be an outstanding call to `begin_debug_group` prior to this one
    /// in the same copy/compute pass or graphics subpass.
    fn end_debug_group(&mut self);

    /// Insert a debug marker.
    ///
    /// A graphics subpass or a copy/compute pass must be active.
    fn insert_debug_marker(&mut self, marker: &DebugMarker);
}

// prevent `InnerXXX` from being exported
mod flags {
    /// Specifies a type of hardware to execute the commands.
    #[derive(EnumFlags, Copy, Clone, Debug, Hash)]
    #[repr(u32)]
    pub enum DeviceEngine {
        /// Generic engine supporting all kinds of commands.
        Universal = 0b0001,

        /// Compute engine supporting compute and copy commands.
        Compute = 0b0010,

        /// Copy engine supporting only copy commands.
        Copy = 0b0100,

        /// The host.
        Host = 0b1000,
    }
}

pub use self::flags::DeviceEngine;
pub type DeviceEngineFlags = BitFlags<DeviceEngine>;
