//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Command queues and command buffers.
//!
//! Command Passes
//! --------------
//!
//! Most commands only can be submitted during a command pass.
//! There are three types of command passes:
//!
//!  - Render pass, only during which render commands defined in [`RenderSubpassCommandEncoder`]
//!    can be encoded. A render pass can be started with [`begin_render_pass`].
//!  - Compute pass, only during which compute commands defined in [`ComputeCommandEncoder`]
//!    can be encoded. A compute pass can be started with [`begin_compute_pass`].
//!  - Copy pass, only during which copy commands defined in [`CopyCommandEncoder`]
//!    can be encoded. A copy pass can be started with [`begin_copy_pass`].
//!
//! After it was started, a command pass is said to be *active* until it is
//! ended by a call to [`end_pass`]. Only one command pass can be active at
//! the same time, so you must ensure to call `end_pass` before starting a new
//! one.
//!
//! Furthermore, a render pass can contain one or more subpasses. During a
//! render pass, [`begin_render_subpass`] and [`end_render_subpass`]
//! must be called for every subpass specified in [`RenderPassDescription`]
//! used to create the [`RenderPass`] associated with the specified
//! [`Framebuffer`].
//!
//! [`RenderSubpassCommandEncoder`]: trait.RenderSubpassCommandEncoder.html
//! [`ComputeCommandEncoder`]: trait.ComputeCommandEncoder.html
//! [`CopyCommandEncoder`]: trait.CopyCommandEncoder.html
//! [`begin_render_pass`]: #tymethod.begin_render_pass
//! [`begin_compute_pass`]: #tymethod.begin_compute_pass
//! [`begin_copy_pass`]: #tymethod.begin_copy_pass
//! [`end_pass`]: #tymethod.end_pass
//! [`begin_render_subpass`]: #tymethod.begin_render_subpass
//! [`end_render_subpass`]: #tymethod.end_render_subpass
//! [`RenderPassDescription`]: ../renderpass/struct.RenderPassDescription.html
//! [`RenderPass`]: ../renderpass/trait.RenderPass.html
//! [`Framebuffer`]: ../framebuffer/trait.Framebuffer.html
//!
//! Engine
//! ------
//!
//! Device engines ([`DeviceEngine`]) represent different parts of the hardware
//! that can process commands concurrently.
//!
//! Every pass is associated with one of device engine other than `Host`.
//!
//! Every subresource can be used by only one device engine at the same time.
//! Also, you need to perform a *engine ownership transfer operation* before
//! using a subresource in a engine other than the engine which was previously
//! accessing the subresource. The engine ownership transfer operation can be
//! performed by a call to [`release_resource`] in the source engine followed by
//! another call to [`acquire_resource`] in the destination engine.
//! You must make sure `acquire_resource` happens-after `release_resource` by
//! using appropriate synchronization primitives (e.g., `Fence` or
//! `CommandBuffer::wait_completion`).
//! If the source or destination engine is `Host` then the corresponding call to
//! `release_resource` or `acquire_resource` (respectively) is not required
//! (in fact, it is impossible since there is no way to start a command pass with
//! the `Host` engine).
//!
//! [`DeviceEngine`]: enum.DeviceEngine.html
//! [`release_resource`]: #tymethod.release_resource
//! [`acquire_resource`]: #tymethod.acquire_resource
//!
use std::fmt::Debug;
use std::any::Any;
use std::ops::Range;

use ngsenumflags::BitFlags;

use {Backend, PipelineStageFlags, DepthBias, DepthBounds, Viewport, Rect2D, Result, Marker,
     ImageSubresourceRange, IndexFormat, ImageLayout, AccessTypeFlags, DebugMarker,
     FenceDescription, DescriptorSetBindingLocation, DeviceSize, VertexBindingLocation,
     ImageAspect, ImageSubresourceLayers};

use cgmath::Vector3;

/// Command queue that accepts and executes command buffers.
///
/// See the [module-level documentation] for more about command buffers.
///
/// [module-level documentation]: ../command/
pub trait CommandQueue<B: Backend>: Debug + Send + Any + Marker {
    fn make_command_buffer(&self) -> Result<B::CommandBuffer>;
    fn make_fence(&self, description: &FenceDescription) -> Result<B::Fence>;

    /// Submit command buffers to a queue.
    ///
    /// The specified command buffers must be in the `Executable` state.
    ///
    /// If `event` is specified, it will be signaled upon completion of
    /// the execution. It must not be associated with any other
    /// commands that has not yet completed execution. It must be in the
    /// unsignalled state.
    ///
    /// After a successful submission, all specified command buffers will be in
    /// the `Pending` or `Completed` state.
    /// If an error occurs during the submission, command buffers will remain
    /// in the original state (`Executable`) and all referenced objects will be
    /// unaffected.
    ///
    fn submit_commands(
        &self,
        buffers: &mut [&mut B::CommandBuffer],
        event: Option<&B::Event>,
    ) -> Result<()>;

    fn wait_idle(&self);
}

/// Command buffer used to record commands to be subsequently submitted to
/// `CommandQueue`.
///
/// When dropping a `CommandBuffer`, it must not be in the `Pending` state.
///
/// See the [module-level documentation] for more about command buffers.
///
/// [module-level documentation]: ../command/
pub trait CommandBuffer<B: Backend>
    : Debug + Send + Any + CommandEncoder<B> + Marker {
    fn state(&self) -> CommandBufferState;

    /// Stall the current thread until the execution of the command buffer
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
    /// Specifies an image subresource, potentially with an image layout transition.
    Image {
        image: &'a B::Image,
        range: ImageSubresourceRange,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
    },

    /// Specifies a buffer subresource.
    Buffer {
        buffer: &'a B::Buffer,
        offset: DeviceSize,
        len: DeviceSize,
    },
}

/// Encodes commands into a command buffer.
///
/// See the [module-level documentation] for more.
///
/// [module-level documentation]: ../command/
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
    ///
    /// # Platform Specific Issues
    ///
    /// - On the Metal backend, there is a limit on the number of command buffers
    ///   waiting to be submitted to the device. Once this limit is reached,
    ///   this method will block until at least one of pending command buffers is
    ///   submitted to the device. The exaxt value appears to be around 64
    ///   command buffers.
    fn begin_encoding(&mut self);

    /// End recording a command buffer. After this function is called,
    /// the command buffer will be moved to the `Executable` state.
    ///
    /// The command buffer must be in the `Recording` state.
    /// No kind of passes can be active at the point of call.
    ///
    /// If there was an error while recording the command buffer, it will be notified
    /// via this function. In such a case, the command buffer will be moved to the `Error`
    /// state.
    fn end_encoding(&mut self) -> Result<()>;

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
}

/// Encodes render commands into a command buffer.
pub trait RenderSubpassCommandEncoder<B: Backend>
    : Debug + Send + Any + DebugCommandEncoder {
    /// Set the current `GraphicsPipeline` object.
    ///
    /// A render pass must be active and compatible with the specified pipeline.
    ///
    /// All dynamic states will be reseted. If the pipeline contains any
    /// dynamic states, their values must be specified via commands before recording any
    /// draw commands. The following shows the correspondences between some fields of
    /// [`GraphicsPipelineRasterizerDescription`], which can be contained in
    /// [`GraphicsPipelineDescription`]`::rasterizer`, and the commands to
    /// specify their values dynamically:
    ///
    ///  - [`set_blend_constants`] - `blend_constants`
    ///  - [`set_depth_bias`] - `depth_bias`
    ///  - [`set_depth_bounds`] - `depth_bounds`
    ///  - [`set_stencil_state`] - `stencil_masks`: This is a special case
    ///    where you have to create a [`StencilState`] with desired values
    ///    before issuing the command
    ///  - [`set_stencil_reference`] - `stencil_references`
    ///  - [`set_viewport`] - `viewport`
    ///  - [`set_scissor_rect`] - `scissor_rect`
    ///
    /// Descriptor sets with incompatible layouts (see Vulkan 1.0 Specification
    /// "13.2.2. Pipeline Layouts") will be unbound and must be bound again
    /// before recording any draw commands.
    ///
    /// [`GraphicsPipelineDescription`]: ../pipeline/struct.GraphicsPipelineDescription.html
    /// [`GraphicsPipelineRasterizerDescription`]: ../pipeline/struct.GraphicsPipelineRasterizerDescription.html
    /// [`StencilState`]: ../pipeline/trait.StencilState.html
    /// [`set_blend_constants`]: #tymethod.set_blend_constants
    /// [`set_depth_bias`]: #tymethod.set_depth_bias
    /// [`set_depth_bounds`]: #tymethod.set_depth_bounds
    /// [`set_stencil_state`]: #tymethod.set_stencil_state
    /// [`set_stencil_reference`]: #tymethod.set_stencil_reference
    /// [`set_viewport`]: #tymethod.set_viewport
    /// [`set_scissor_rect`]: #tymethod.set_scissor_rect
    fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline);

    /// Specify the dynamic blend constant values.
    ///
    /// The current `GraphicsPipeline` must have been created with rasterization
    /// enabled and `GraphicsPipelineRasterizerDescription::blend_constants`
    /// set to `StaticOrDynamic::Dynamic`.
    fn set_blend_constants(&mut self, value: &[f32; 4]);

    /// Specify the dynamic depth bias values.
    ///
    /// The current `GraphicsPipeline` must have been created with rasterization
    /// enabled and `GraphicsPipelineRasterizerDescription::depth_bias`
    /// set to `StaticOrDynamic::Dynamic`.
    fn set_depth_bias(&mut self, value: Option<DepthBias>);

    /// Specify the dynamic depth bound values.
    ///
    /// The current `GraphicsPipeline` must have been created with rasterization
    /// enabled and `GraphicsPipelineRasterizerDescription::depth_bounds`
    /// set to `StaticOrDynamic::Dynamic`.
    fn set_depth_bounds(&mut self, value: Option<DepthBounds>);

    /// Sets the current `StencilState` object.
    ///
    /// The specified `StencilState` must have been created with
    /// `StencilStateDescription::pipeline` set to the same pipeline as the
    /// currently active one.
    fn set_stencil_state(&mut self, value: &B::StencilState);

    /// Set the current stencil reference values for the front-facing primitives and
    /// back-facing ones, respectively.
    ///
    /// The current `GraphicsPipeline` must have been created with rasterization
    /// enabled and `GraphicsPipelineRasterizerDescription::stencil_references`
    /// set to `StaticOrDynamic::Dynamic`.
    fn set_stencil_reference(&mut self, values: [u32; 2]);

    /// Specify the dynamic viewport values.
    ///
    /// The current `GraphicsPipeline` must have been created with rasterization
    /// enabled and `GraphicsPipelineRasterizerDescription::viewport`
    /// set to `StaticOrDynamic::Dynamic`.
    fn set_viewport(&mut self, value: &Viewport);

    /// Specify the dynamic scissor rectangle.
    ///
    /// The current `GraphicsPipeline` must have been created with rasterization
    /// enabled and `GraphicsPipelineRasterizerDescription::scissor_rect`
    /// set to `StaticOrDynamic::Dynamic`.
    ///
    /// All coordinate values must lie in the range `[0, i32::max_value()]`.
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

    /// Render primitives.
    ///
    /// `vertex_range` specifies the consecutive range of vertex indices to draw.
    ///
    /// The primitives are drawn for `instance_range.len()` times.
    /// Specify `0..1` to perform a normal (not instanced) rendering.
    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>);

    /// Render primitives using a currently bound index buffer.
    ///
    /// Vertex indices are retrieved from the consecutive range of index buffer
    /// specified by `index_buffer_range`.
    /// Before indexing into the vertex buffers, the value of `vertex_offset` is
    /// added to the vertex index.
    ///
    /// The primitives are drawn for `instance_range.len()` times. Specify `0..1`
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
    /// Fill a buffer with a constant byte value.
    ///
    /// Each of `range.start` and `range.end` must be a multiple of 4.
    fn fill_buffer(&mut self, destination: &B::Buffer, range: Range<DeviceSize>, value: u8);

    /// Copy data from a buffer to another buffer.
    ///
    /// Each of `source_offset`, `destination_offset`, and `size` must be a
    /// multiple of 4.
    fn copy_buffer(
        &mut self,
        source: &B::Buffer,
        source_offset: DeviceSize,
        destination: &B::Buffer,
        destination_offset: DeviceSize,
        size: DeviceSize,
    );

    /// Copy data from a buffer to an image.
    ///
    /// The image must be in the `General` or `TransferDestination` layout.
    ///
    /// If the image has a depth/stencil format, the current device engine must
    /// be `DeviceEngine::Universal`.
    fn copy_buffer_to_image(
        &mut self,
        source: &B::Buffer,
        source_range: &BufferImageRange,
        destination: &B::Image,
        destination_layout: ImageLayout,
        destination_aspect: ImageAspect,
        destination_subresource_range: &ImageSubresourceLayers,
        destination_origin: Vector3<u32>,
        size: Vector3<u32>,
    );

    /// Copy data from an image to an buffer.
    ///
    /// The image must be in the `General` or `TransferSource` layout.
    ///
    /// If the image has a depth/stencil format, the current device engine must
    /// be `DeviceEngine::Universal`.
    fn copy_image_to_buffer(
        &mut self,
        source: &B::Image,
        source_layout: ImageLayout,
        source_aspect: ImageAspect,
        source_subresource_range: &ImageSubresourceLayers,
        source_origin: Vector3<u32>,
        destination: &B::Buffer,
        destination_range: &BufferImageRange,
        size: Vector3<u32>,
    );

    /// Copy data from an image to another image.
    ///
    /// The source image must be in the `General` or `TransferSource` layout.
    /// The destination must be in the `General` or `TransferDestination` layout.
    ///
    /// The source and destination images must have the same image format and
    /// the same sample count.
    ///
    /// `source_subresource_range` and `destination_subresource_range` must have
    /// the same number of array layers.
    fn copy_image(
        &mut self,
        source: &B::Image,
        source_layout: ImageLayout,
        source_subresource_range: &ImageSubresourceLayers,
        source_origin: Vector3<u32>,
        destination: &B::Image,
        destination_layout: ImageLayout,
        destination_subresource_range: &ImageSubresourceLayers,
        destination_origin: Vector3<u32>,
        size: Vector3<u32>,
    );
}

/// Specifies the layout of an image data in a buffer.
pub struct BufferImageRange {
    /// Offset (in bytes) of the image data from the start of the buffer.
    ///
    /// Must be a multiple of 4 and the image's pixel size.
    pub offset: DeviceSize,

    /// Strides (in pixels) between rows of the buffer data.
    ///
    /// Must be less than or equal to 32767.
    pub row_stride: DeviceSize,

    /// Strides (in pixels) between 2D images of the buffer data.
    ///
    /// Must be less than `2^32`.
    pub plane_stride: DeviceSize,
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

/// Specifies a type of hardware to execute the commands.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
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

pub type DeviceEngineFlags = BitFlags<DeviceEngine>;
