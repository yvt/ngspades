//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Command queues and command buffers.
use bitflags::bitflags;
use flags_macro::flags;
use std::ops::Range;
use std::sync::Arc;

use crate::formats::IndexFormat;
use crate::resources::{BufferRef, ImageLayout, ImageRef, ImageSubRange};
use crate::{arg, heap, pass, pipeline, resources, sync};
use crate::{
    AccessTypeFlags, ArgTableIndex, DeviceSize, QueueFamily, StageFlags, VertexBufferIndex,
    Viewport, ViewportIndex,
};
use crate::{Object, Result};
use zangfx_common::Rect2D;

/// A builder object for command queue objects.
pub type CmdQueueBuilderRef = Box<dyn CmdQueueBuilder>;

/// Trait for building command queue objects.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device) {
///     let cmd_queue = device.build_cmd_queue()
///         .queue_family(0)
///         .build()
///         .expect("Failed to create a command queue.");
///     # }
///
pub trait CmdQueueBuilder: Object {
    /// Set the queue family index.
    ///
    /// This property is mandatory.
    fn queue_family(&mut self, v: QueueFamily) -> &mut dyn CmdQueueBuilder;

    /// Build a `CmdQueue`.
    ///
    /// # Valid Usage
    ///
    /// - All mandatory properties must have their values set before this method
    ///   is called.
    /// - For each queue family, the number of command queues created from
    ///   a device (including those already dropped) must be less than or equal
    ///   to [`QueueFamilyInfo::count`].
    ///
    /// [`QueueFamilyInfo::count`]: crate::limits::QueueFamilyInfo::count
    ///
    fn build(&mut self) -> Result<CmdQueueRef>;
}

/// A boxed handle representing a command queue.
pub type CmdQueueRef = Arc<dyn CmdQueue>;

/// Trait for command queues.
///
/// The lifetime of the underlying queue object is associated with that of
/// `CmdQueue`. Drop the `CmdQueue` to destroy the associated queue object (cf.
/// handle types).
///
/// # Valid Usage
///
///  - `CmdQueue` must not be dropped until the queue is idle. (i.e. There
///    exists no command buffer being executed)
///
pub trait CmdQueue: Object {
    /// Allocate a new command buffer.
    ///
    /// Command buffers are meant to be shortly lived. This method might stall
    /// if there are too many (10–) outstanding command buffers.
    fn new_cmd_buffer(&self) -> Result<CmdBufferRef>;

    /// Create a `FenceRef` associated with the command queue.
    fn new_fence(&self) -> Result<sync::FenceRef>;

    /// Schedule pending commited command buffers for execution.
    fn flush(&self);
}

/// A command buffer.
pub type CmdBufferRef = Box<dyn CmdBuffer>;

/// Trait for command buffers.
///
/// An application can (and should) drop a `CmdBuffer` object as soon as
/// it finishes recording commands and commiting it.
pub trait CmdBuffer: Object {
    /// Mark this command buffer as ready for submission.
    ///
    /// This method essentially (but no in terms of the Rust language semantics
    /// because doing it through `Box<dyn _>` is currently impossible, although
    /// there is [a PR] to make doing such things possible) consumes the command
    /// buffer object, so you won't be able to call any `CmdBuffer` methods
    /// after calling this.
    ///
    /// [a PR]: https://github.com/rust-lang/rust/pull/54183
    ///
    /// # Valid Usage
    ///
    /// - On a command buffer object, no methods of `CmdBuffer` may be called
    ///   after this method is called.
    ///
    fn commit(&mut self) -> Result<()>;

    /// Begin encoding a render pass.
    ///
    /// # Valid Usage
    ///
    /// - All images in `render_target_table` must be associated with the queue
    ///   to which this command buffer belongs.
    ///
    fn encode_render(
        &mut self,
        render_target_table: &pass::RenderTargetTableRef,
    ) -> &mut dyn RenderCmdEncoder;
    /// Begin encoding a compute pass.
    fn encode_compute(&mut self) -> &mut dyn ComputeCmdEncoder;
    /// Begin encoding a copy pass.
    fn encode_copy(&mut self) -> &mut dyn CopyCmdEncoder;

    /// Register a completion handler.
    ///
    /// Note that this method may not be called after `commit` is called.
    fn on_complete(&mut self, cb: Box<dyn FnMut(Result<()>) + Sync + Send>);

    /// Wait on a given semaphore before the execution of the command buffer.
    ///
    /// The default implementation panics.
    fn wait_semaphore(&mut self, semaphore: &sync::SemaphoreRef, dst_stage: StageFlags) {
        let _ = (semaphore, dst_stage);
        panic!("Semaphores are not supported by this backend.");
    }

    /// Signal a given semaphore after the execution of the command buffer.
    ///
    /// The default implementation panics.
    fn signal_semaphore(&mut self, semaphore: &sync::SemaphoreRef, src_stage: StageFlags) {
        let _ = (semaphore, src_stage);
        panic!("Semaphores are not supported by this backend.");
    }

    /// Make device writes to buffers done during the execution of this and
    /// preceding command buffers visible to the host.
    ///
    /// The opposite, acquiring operation is not required. (See Vulkan 1.0
    /// "6.9. Host Write Ordering Guarantees". On Metal, both operations are
    /// implicit)
    ///
    /// The default implementation is no-op.
    ///
    /// # Valid Usage
    ///
    /// - All buffers in `buffers` must be associated with the queue to which
    ///   this command buffer belongs.
    ///
    fn host_barrier(
        &mut self,
        src_access: AccessTypeFlags,
        buffers: &[(Range<DeviceSize>, &resources::BufferRef)],
    ) {
        let _ = (src_access, buffers);
    }

    /// Invalidate the contents of a given images, causing reinitialization of
    /// the contents in a subsequent use of the image.
    ///
    /// This operation affects every [state-tracking unit] intersecting with
    /// `images`.
    ///
    /// This method is used in the following scenarios:
    ///
    ///  - The old contents of images are no longer required in following passes.
    ///  - The memory representation of images might be in an invalid state due
    ///    to aliasing.
    ///
    /// The `DontCare` load action of a render pass target has the same effect
    /// as `invalidate_image`.
    ///
    /// # Valid Usage
    ///
    /// - All images in `images` must be associated with the queue to which
    ///   this command buffer belongs.
    ///
    /// [state-tracking unit]: crate::Image
    fn invalidate_image(&mut self, images: &[&resources::ImageRef]) {
        let _ = images;
    }

    /// Acquire resources from another queue with a different queue family.
    ///
    /// For images, this operation affects every [state-tracking unit]
    /// intersecting with given `&ImageRef`s.
    ///
    /// This operation is a part of a queue family ownership transfer operation.
    /// See Vulkan 1.0 "6.7.4. Queue Family Ownership Transfer" for details.
    /// The sending end and receiving end must call `queue_ownership_acquire` and
    /// `queue_ownership_release` respectively, using an identical
    /// `QueueOwnershipTransfer` value.
    ///
    /// The default implementation panics. Implementations that support more
    /// than one queue families must override this method.
    ///
    /// # Valid Usage
    ///
    /// - All resources in `tranfer` must be associated with the queue to which
    ///   this command buffer belongs.
    ///
    /// [state-tracking unit]: crate::Image
    fn queue_ownership_acquire(
        &mut self,
        src_queue_family: QueueFamily,
        dst_access: AccessTypeFlags,
        transfer: &[QueueOwnershipTransfer<'_>],
    ) {
        let _ = (src_queue_family, dst_access, transfer);
        panic!("Queue families are not supported by this backend.");
    }

    /// Release resources from another queue with a different queue family.
    ///
    /// For images, this operation affects every [state-tracking unit]
    /// intersecting with given `&ImageRef`s.
    ///
    /// This operation is a part of a queue family ownership transfer operation.
    /// See Vulkan 1.0 "6.7.4. Queue Family Ownership Transfer" for details.
    /// The sending end and receiving end must call `queue_ownership_acquire` and
    /// `queue_ownership_release` respectively, using an identical
    /// `QueueOwnershipTransfer` value.
    ///
    /// The default implementation panics. Implementations that support more
    /// than one queue families must override this method.
    ///
    /// # Valid Usage
    ///
    /// - All resources in `tranfer` must be associated with the queue to which
    ///   this command buffer belongs.
    ///
    /// [state-tracking unit]: crate::Image
    fn queue_ownership_release(
        &mut self,
        dst_queue_family: QueueFamily,
        src_access: AccessTypeFlags,
        transfer: &[QueueOwnershipTransfer<'_>],
    ) {
        let _ = (dst_queue_family, src_access, transfer);
        panic!("Queue families are not supported by this backend.");
    }
}

pub trait RenderCmdEncoder: Object + CmdEncoder {
    /// Set the current `RenderPipelineRef` object.
    ///
    /// All non-dynamic state values of the new `RenderPipelineRef` will override
    /// the current ones. Other states are left intact.
    fn bind_pipeline(&mut self, pipeline: &pipeline::RenderPipelineRef);

    /// Set the blend constant values.
    ///
    /// # Valid Usage
    ///
    /// `value` must have exactly four elements.
    fn set_blend_constant(&mut self, value: &[f32]);

    /// Specify the dynamic depth bias values.
    ///
    /// # Valid Usage
    ///
    /// The current `RenderPipelineRef` must have been created with rasterization
    /// enabled and `RenderPassRasterizer::set_depth_bias` called with
    /// `Some(Dynamic(_))`.
    fn set_depth_bias(&mut self, value: Option<pipeline::DepthBias>);

    /// Specify the dynamic depth bound values.
    ///
    /// # Valid Usage
    ///
    /// The current `RenderPipelineRef` must have been created with rasterization
    /// enabled and `RenderPassRasterizer::set_depth_bounds` called with
    /// `Some(Dynamic(_))`.
    ///
    fn set_depth_bounds(&mut self, value: Option<Range<f32>>);

    /// Set the current stencil reference values for the front-facing primitives
    /// and back-facing ones, respectively.
    ///
    /// `value` must have exactly two elements.
    fn set_stencil_refs(&mut self, values: &[u32]);

    /// Specify the dynamic viewport values.
    fn set_viewports(&mut self, start_viewport: ViewportIndex, value: &[Viewport]);

    /// Specify the dynamic scissor rects.
    ///
    /// # Valid Usage
    ///
    /// The current `RenderPipelineRef` must have been created with rasterization
    /// enabled and `RenderPassRasterizer::set_scissors` called with
    /// `Dynamic(_)` for the corresponding viewports.
    fn set_scissors(&mut self, start_viewport: ViewportIndex, value: &[Rect2D<u32>]);

    /// Bind zero or more `ArgTableRef`s.
    ///
    /// # Valid Usage
    ///
    /// - All argument pools in `tables` must be associated with the queue to
    ///   which this command buffer belongs.
    /// - All argument table in `tables` must originate from their respective
    ///   argument pools.
    ///
    fn bind_arg_table(
        &mut self,
        index: ArgTableIndex,
        tables: &[(&arg::ArgPoolRef, &arg::ArgTableRef)],
    );

    /// Bind zero or more vertex buffers.
    ///
    /// # Valid Usage
    ///
    /// - All buffers in `buffers` must be associated with the queue to which
    ///   this command buffer belongs.
    ///
    fn bind_vertex_buffers(
        &mut self,
        index: VertexBufferIndex,
        buffers: &[(&resources::BufferRef, DeviceSize)],
    );

    /// Bind an index buffer.
    ///
    /// # Valid Usage
    ///
    /// - `buffer` must be associated with the queue to which this command
    ///   buffer belongs.
    fn bind_index_buffer(
        &mut self,
        buffer: &resources::BufferRef,
        offset: DeviceSize,
        format: IndexFormat,
    );

    /// Render primitives.
    ///
    /// `vertex_range` specifies the consecutive range of vertex indices to draw.
    ///
    /// The primitives are drawn for `instance_range.len()` times.
    /// Specify `0..1` to perform a normal (not instanced) rendering.
    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>);

    /// Render primitives using the currently bound index buffer.
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

    /// Render primitives. Parameters are read by the device from a buffer.
    ///
    /// The draw parameters are defined by [`DrawIndirectArgs`].
    ///
    /// # Valid Usage
    ///
    /// - `offset` must be aligned to 4 bytes.
    /// - `buffer` must be associated with the queue to which this command
    ///   buffer belongs.
    ///
    /// [`DrawIndirectArgs`]: DrawIndirectArgs
    fn draw_indirect(&mut self, buffer: &resources::BufferRef, offset: DeviceSize);

    /// Render primitives using the currently bound index buffer. Parameters are
    /// read by the device from a buffer.
    ///
    /// The draw parameters are defined by [`DrawIndexedIndirectArgs`].
    ///
    /// # Valid Usage
    ///
    /// - `offset` must be aligned to 4 bytes.
    /// - `buffer` must be associated with the queue to which this command
    ///   buffer belongs.
    ///
    /// [`DrawIndexedIndirectArgs`]: DrawIndexedIndirectArgs
    fn draw_indexed_indirect(&mut self, buffer: &resources::BufferRef, offset: DeviceSize);
}

/// The data layout for indirect draw calls.
#[repr(C)]
pub struct DrawIndirectArgs {
    /// The number of vertices to draw.
    pub num_vertices: u32,
    /// THe number of instances to draw.
    pub num_instances: u32,
    /// The first vertex index to draw.
    pub start_vertex: u32,
    /// The first instance index to draw.
    pub start_instance: u32,
}

/// The data layout for indexed indirect draw calls.
#[repr(C)]
pub struct DrawIndexedIndirectArgs {
    /// The number of vertices to draw.
    pub num_vertices: u32,
    /// THe number of instances to draw.
    pub num_instances: u32,
    /// The first index within the index buffer.
    pub start_index: u32,
    /// The value added before indexing into the vertxe buffer.
    pub vertex_offset: u32,
    /// The first instance index to draw.
    pub start_instance: u32,
}

pub trait ComputeCmdEncoder: Object + CmdEncoder {
    /// Set the current `ComputePipelineRef` object.
    fn bind_pipeline(&mut self, pipeline: &pipeline::ComputePipelineRef);

    /// Bind zero or more `ArgTableRef`s.
    ///
    /// # Valid Usage
    ///
    /// - All argument pools in `tables` must be associated with the queue to
    ///   which this command buffer belongs.
    /// - All argument table in `tables` must originate from their respective
    ///   argument pools.
    ///
    fn bind_arg_table(
        &mut self,
        index: ArgTableIndex,
        tables: &[(&arg::ArgPoolRef, &arg::ArgTableRef)],
    );

    /// Provoke work in a compute pipeline.
    ///
    /// `workgroup_count` is an array with up to 3 elements. When less than
    /// 3 elements are given, the missing ones are filled with `1`s.
    ///
    /// # Valid Usage
    ///
    ///  - `workgroup_count` must not exceed the hardware limit indicated by
    ///    [`DeviceLimits::max_compute_workgroup_count`].
    ///
    /// [`DeviceLimits::max_compute_workgroup_count`]: crate::limits::DeviceLimits::max_compute_workgroup_count
    fn dispatch(&mut self, workgroup_count: &[u32]);

    /// Provoke work in a compute pipeline. Parameters are read by the device
    /// from a buffer.
    ///
    /// The draw parameters are defined by [`DispatchIndirectArgs`].
    ///
    /// # Valid Usage
    ///
    /// - `offset` must be aligned to 4 bytes.
    /// - `buffer` must be associated with the queue to which this command
    ///   buffer belongs.
    ///
    /// [`DispatchIndirectArgs`]: crate::command::DispatchIndirectArgs
    fn dispatch_indirect(&mut self, buffer: &resources::BufferRef, offset: DeviceSize);
}

/// The data layout for indirect dispatch calls.
pub type DispatchIndirectArgs = [u32; 3];

pub trait CopyCmdEncoder: Object + CmdEncoder {
    /// Fill a buffer with a constant byte value.
    ///
    /// Both of `range.start` and `range.end` must be a multiple of 4.
    ///
    /// # Valid Usage
    ///
    /// - `buffer` must be associated with the queue to which this command
    ///   buffer belongs.
    ///
    fn fill_buffer(&mut self, buffer: &resources::BufferRef, range: Range<DeviceSize>, value: u8);

    /// Copy data from a buffer to another buffer.
    ///
    /// All of `source_offset`, `destination_offset`, and `size` must be a
    /// multiple of 4.
    ///
    /// # Valid Usage
    ///
    /// - `buffer` must be associated with the queue to which this command
    ///   buffer belongs.
    ///
    fn copy_buffer(
        &mut self,
        src: &resources::BufferRef,
        src_offset: DeviceSize,
        dst: &resources::BufferRef,
        dst_offset: DeviceSize,
        size: DeviceSize,
    );

    /// Copy data from a buffer to an image.
    ///
    /// The image must be in the `General` or `TransferDestination` layout.
    ///
    /// If the image has a depth/stencil format, the current command queue must
    /// support graphics operations.
    ///
    /// If `dst_origin` has fewer elements than the dimensionality of the
    /// destination image, the rest is assumed to be all `0`.
    ///
    /// If `size` has fewer elements than the dimensionality of the
    /// destination image, the rest is assumed to be all `1`.
    ///
    /// # Valid Usage
    ///
    /// - `src` and `dst` must be associated with the queue to which this
    ///   command buffer belongs.
    /// - `dst_aspect` must be contained by the destination image `dst`.
    ///
    fn copy_buffer_to_image(
        &mut self,
        src: &resources::BufferRef,
        src_range: &BufferImageRange,
        dst: &resources::ImageRef,
        dst_aspect: resources::ImageAspect,
        dst_range: &resources::ImageLayerRange,
        dst_origin: &[u32],
        size: &[u32],
    );

    /// Copy data from an image to an buffer.
    ///
    /// The image must be in the `General` or `TransferSource` layout.
    ///
    /// If the image has a depth/stencil format, the current command queue must
    /// support graphics operations.
    ///
    /// If `src_origin` has fewer elements than the dimensionality of the
    /// source image, the rest is assumed to be all `0`.
    ///
    /// If `size` has fewer elements than the dimensionality of the
    /// source image, the rest is assumed to be all `1`.
    ///
    /// # Valid Usage
    ///
    /// - `src` and `dst` must be associated with the queue to which this
    ///   command buffer belongs.
    /// - `src_aspect` must be contained by the source image `src`.
    ///
    fn copy_image_to_buffer(
        &mut self,
        src: &resources::ImageRef,
        src_aspect: resources::ImageAspect,
        src_range: &resources::ImageLayerRange,
        src_origin: &[u32],
        dst: &resources::BufferRef,
        dst_range: &BufferImageRange,
        size: &[u32],
    );

    /// Copy data from an image to another image.
    ///
    /// The source image must be in the `General` or `CopyRead` layout.
    /// The destination must be in the `General` or `CopyWrite` layout.
    ///
    /// The source and destination images must have the same image format and
    /// the same sample count.
    ///
    /// `src_range` and `dst_range` must have the same number of array layers.
    ///
    /// If `src_origin` has fewer elements than the dimensionality of the
    /// source image, the rest is assumed to be all `0`. Similarly, if
    /// `dst_origin` has fewer elements than the dimensionality of the
    /// destination image, the rest is assumed to be all `0`.
    ///
    /// If `size` has fewer elements than the dimensionality of the
    /// source and/or destination image, the rest is assumed to be all `1`.
    ///
    /// # Valid Usage
    ///
    /// - `src` and `dst` must be associated with the queue to which this
    ///   command buffer belongs.
    ///
    fn copy_image(
        &mut self,
        src: &resources::ImageRef,
        src_range: &resources::ImageLayerRange,
        src_origin: &[u32],
        dst: &resources::ImageRef,
        dst_range: &resources::ImageLayerRange,
        dst_origin: &[u32],
        size: &[u32],
    );
}

pub trait CmdEncoder: Object {
    /// Begin a debug group.
    ///
    /// The default implementation just returns `None`.
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::*;
    ///     # fn test(encoder: &mut CmdEncoder) {
    ///     encoder.begin_debug_group("Pinkie mane");
    ///     // Issue draw commands here...
    ///     encoder.end_debug_group();
    ///     # }
    ///
    fn begin_debug_group(&mut self, _label: &str) {}

    /// End a debug group.
    ///
    /// There must be an outstanding call to [`begin_debug_group`] corresponding
    /// to this one in the same encoder.
    ///
    /// [`begin_debug_group`]: CmdEncoder::begin_debug_group
    fn end_debug_group(&mut self) {}

    /// Insert a debug marker.
    ///
    /// The default implementation just returns `None`.
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::*;
    ///     # fn test(encoder: &mut CmdEncoder) {
    ///     encoder.debug_marker("Let there be dragons here");
    ///     # }
    ///
    fn debug_marker(&mut self, _label: &str) {}

    /// Declare that the specified resources are referenced by the descriptor
    /// sets used on this command encoder.
    ///
    /// See [`CmdEncoderExt::use_resource`] for an ergonomic wrapper of this method.
    ///
    /// This ensures the resources are resident starting from the point where
    /// this command is inserted and until the end of the current command
    /// encoder or subpass. You must call this method for every resource
    /// indirectly referenced by argument tables.
    ///
    /// If you have an image and image view created from it, calling this method
    /// only on the image does not make the metadata of the image view resident.
    ///
    /// This method is practically no-op on `CopyCmdEncoder` since it does not
    /// use any argument tables, although it may incur a run-time overhead.
    ///
    /// # Valid Usage
    ///
    /// - All resources in `objs` must be associated with the queue to which
    ///   this command buffer belongs.
    /// - If `self` is a render command encoder, `objs` must not overlap with
    ///   its render targets.
    ///
    fn use_resource_core(&mut self, usage: ResourceUsageFlags, objs: resources::ResourceSet<'_>);

    /// Declare that the resources in the specified heaps are referenced by the
    /// argument tables used on this command encoder.
    ///
    /// This ensures the resources are resident starting from the point where
    /// this command is inserted and until the end of the current command
    /// encoder or subpass.
    ///
    /// This method is no-op on `CopyCmdEncoder` since it does not use any
    /// argument tables.
    ///
    /// This method only can be used on dedicated heaps.
    ///
    /// This method ignores images having [`Render`] or [`Storage`] image usage
    /// flags. Call [`use_resource`] instead to use such images.
    ///
    /// [`Device::global_heap`]: crate::Device::global_heap
    /// [`DynamicHeapBuilder`]: crate::heap::DynamicHeapBuilder
    /// [`Render`]: crate::resources::ImageUsageFlags::Render
    /// [`Storage`]: crate::resources::ImageUsageFlags::Storage
    /// [`use_resource`]: crate::command::CmdEncoderExt::use_resource
    ///
    /// # Valid Usage
    ///
    ///  - Every heap in `heaps` must be a dedicated heap (created via
    ///    [`crate::heap::DedicatedHeapBuilder`]) that have `use_heap` enabled
    ///    on them (by calling
    ///    [`crate::heap::DedicatedHeapBuilder::enable_use_heap`]).
    ///  - All heaps in `heaps` must be associated with the queue to which
    ///    this command buffer belongs.
    ///
    fn use_heap(&mut self, heaps: &[&heap::HeapRef]);

    /// Wait on the specified fence and establish an inter-encoder execution
    /// dependency.
    ///
    /// The fence must be updated first before waiting on it. The command queue
    /// automatically reorders command buffer submissions to satisfy this
    /// constraint. If fence operations are inserted in a way there exists no
    /// such ordering, a dead-lock might occur.
    ///
    /// # Valid Usage
    ///
    /// - `fence` must be associated with the queue to which this command buffer
    ///   belongs.
    ///
    fn wait_fence(&mut self, fence: &sync::FenceRef, dst_access: AccessTypeFlags);

    /// Update the specified fence.
    ///
    /// A fence can be updated only once. You must create a new one after done
    /// using the old one.
    ///
    /// # Valid Usage
    ///
    /// - `fence` must be associated with the queue to which this command buffer
    ///   belongs.
    /// - If this command buffer is to be commited in the future at some point
    ///   in the future, the submission of this command buffer must not cause
    ///   `fence` to be updated more than once.
    ///
    fn update_fence(&mut self, fence: &sync::FenceRef, src_access: AccessTypeFlags);

    /// Insert a barrier and establish an execution dependency within the
    /// current encoder or subpass.
    ///
    /// See [`CmdEncoderExt::barrier`] for an ergonomic wrapper of this method.
    ///
    /// When this is called inside a render subpass, a self-dependency with
    /// matching access type flags and stage flags must have been defined on the
    /// subpass.
    ///
    /// [`CmdEncoderExt::barrier`]: crate::command::CmdEncoderExt::barrier
    ///
    /// # Valid Usage
    ///
    /// - All resources in `obj` must be associated with the queue to which
    ///   this command buffer belongs.
    ///
    fn barrier_core(
        &mut self,
        obj: resources::ResourceSet<'_>,
        src_access: AccessTypeFlags,
        dst_access: AccessTypeFlags,
    );
}

/// Utilies for [`CmdEncoder`].
pub trait CmdEncoderExt: CmdEncoder {
    /// Declare that the specified resources are referenced by the descriptor
    /// sets used on this command encoder.
    ///
    /// This is an ergonomic wrapper for [`CmdEncoder::use_resource_core`].
    ///
    /// See also [`CmdEncoderExt::use_resource_read`] and
    /// [`CmdEncoderExt::use_resource_read_write`].
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::*;
    ///     use flags_macro::flags;
    ///
    ///     # fn test(encoder: &mut CmdEncoder, image: ImageRef, buffer: BufferRef) {
    ///     // Single resource
    ///     encoder.use_resource(
    ///         flags![ResourceUsageFlags::{READ | SAMPLE}],
    ///         &image,
    ///     );
    ///     encoder.use_resource(
    ///         flags![ResourceUsageFlags::{READ | SAMPLE}],
    ///         &buffer,
    ///     );
    ///
    ///     // Homogeneous list
    ///     encoder.use_resource(
    ///         flags![ResourceUsageFlags::{READ | SAMPLE}],
    ///         &[&image, &image][..],
    ///     );
    ///     encoder.use_resource(
    ///         flags![ResourceUsageFlags::{READ | SAMPLE}],
    ///         &[&buffer, &buffer][..],
    ///     );
    ///
    ///     // Heterogeneous list
    ///     encoder.use_resource(
    ///         flags![ResourceUsageFlags::{READ | SAMPLE}],
    ///         &resources![&image, &buffer][..],
    ///     );
    ///     # }
    ///
    /// # Valid Usage
    ///
    /// See [`CmdEncoder::use_resource_core`].
    fn use_resource<'a, T: Into<resources::ResourceSet<'a>>>(
        &mut self,
        usage: ResourceUsageFlags,
        objs: T,
    ) {
        self.use_resource_core(usage, objs.into())
    }

    /// Declare that the specified resources are referenced by the descriptor
    /// sets used on this command encoder. The usage is limited to read
    /// accesses (`flags![ResourceUsageFlags::{READ | SAMPLE}]`).
    ///
    /// This is an ergonomic wrapper for [`CmdEncoder::use_resource_core`].
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::*;
    ///     # fn test(encoder: &mut CmdEncoder, image: ImageRef, buffer: BufferRef) {
    ///     // Single resource
    ///     encoder.use_resource_read(&image);
    ///     encoder.use_resource_read(&buffer);
    ///
    ///     // Homogeneous list
    ///     encoder.use_resource_read(&[&image, &image][..]);
    ///     encoder.use_resource_read(&[&buffer, &buffer][..]);
    ///
    ///     // Heterogeneous list
    ///     encoder.use_resource_read(&resources![&image, &buffer][..]);
    ///     # }
    ///
    /// # Valid Usage
    ///
    /// See [`CmdEncoder::use_resource_core`].
    fn use_resource_read<'a, T: Into<resources::ResourceSet<'a>>>(&mut self, objs: T) {
        self.use_resource(flags![ResourceUsageFlags::{READ | SAMPLE}], objs)
    }

    /// Declare that the specified resources are referenced by the descriptor
    /// sets used on this command encoder. All accesses are allowed
    /// (`flags![ResourceUsageFlags::{READ | WRITE | SAMPLE}]`).
    ///
    /// This is an ergonomic wrapper for [`CmdEncoder::use_resource_core`].
    ///
    /// # Examples
    ///
    /// See [`CmdEncoderExt::use_resource_read`].
    ///
    /// # Valid Usage
    ///
    /// See [`CmdEncoder::use_resource_core`].
    fn use_resource_read_write<'a, T: Into<resources::ResourceSet<'a>>>(&mut self, objs: T) {
        self.use_resource(flags![ResourceUsageFlags::{READ | WRITE | SAMPLE}], objs)
    }

    /// Insert a barrier and establish an execution dependency within the
    /// current encoder or subpass.
    ///
    /// This is an ergonomic wrapper for [`CmdEncoder::barrier_core`].
    ///
    /// # Valid Usage
    ///
    /// See [`CmdEncoder::barrier_core`].
    fn barrier<'a, T: Into<resources::ResourceSet<'a>>>(
        &mut self,
        obj: T,
        src_access: AccessTypeFlags,
        dst_access: AccessTypeFlags,
    ) {
        self.barrier_core(obj.into(), src_access, dst_access)
    }
}

impl<T: ?Sized + CmdEncoder> CmdEncoderExt for T {}

bitflags! {
    /// Describes how a resource will be used in a shader.
    pub struct ResourceUsageFlags: u8 {
        /// Enables reading from the resource via arguments of the [`StorageImage`],
        /// [`UniformBuffer`], or [`StorageBuffer`] types.
        ///
        /// [`StorageImage`]: crate::ArgType::StorageImage
        /// [`UniformBuffer`]: crate::ArgType::UniformBuffer
        /// [`StorageBuffer`]: crate::ArgType::StorageBuffer
        const READ = 0b001;
        /// Enables writing to the resource via arguments of the [`StorageImage`],
        /// or [`StorageBuffer`] types.
        ///
        /// [`StorageImage`]: crate::ArgType::StorageImage
        /// [`StorageBuffer`]: crate::ArgType::StorageBuffer
        const WRITE = 0b010;
        /// Enables texture sampling from the resource via arguments of the
        /// [`SampledImage`] type.
        ///
        /// [`SampledImage`]: crate::ArgType::SampledImage
        const SAMPLE = 0b100;
    }
}

/// Specifies the layout of an image data in a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Must be less than `1<<32`.
    pub plane_stride: DeviceSize,
}

/// Describes a queue family ownership transfer operation.
#[derive(Debug, Clone)]
pub enum QueueOwnershipTransfer<'a> {
    Buffer {
        buffer: &'a BufferRef,
        range: Option<Range<DeviceSize>>,
    },
    Image {
        image: &'a ImageRef,
        src_layout: ImageLayout,
        dst_layout: ImageLayout,
        range: ImageSubRange,
    },
}
