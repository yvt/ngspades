//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Command queues and command buffers.
use std::ops::Range;

use Object;
use common::{Rect2D, Result};
use {handles, heap, pipeline, resources};
use {AccessTypeFlags, ArgTableIndex, DeviceSize, QueueFamily, StageFlags, VertexBufferIndex,
     Viewport, ViewportIndex};
use formats::IndexFormat;

/// Trait for building command queue objects.
///
/// # Valid Usage
///
///  - No instance of `CmdQueueBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
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
    fn queue_family(&mut self, v: QueueFamily) -> &mut CmdQueueBuilder;

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
    /// [`QueueFamilyInfo::count`]: QueueFamilyInfo::count
    ///
    fn build(&mut self) -> Result<Box<CmdQueue>>;
}

/// Trait for command queues.
///
/// The lifetime of the underlying queue object is associated with that of
/// `CmdQueue`. Drop the `CmdQueue` to destroy the associated queu object (cf.
/// handle types).
///
/// # Valid Usage
///
///  - No instance of `CmdQueue` may outlive the originating `Device`.
///  - `CmdQueue` must not be dropped until the queue is idle. (i.e. There
///    exists no command buffer being executed)
///
pub trait CmdQueue: Object {
    /// Allocate a new command buffer.
    ///
    /// Command buffers are meant to be shortly lived. This method might stall
    /// if there are too many (50–) pending command buffers.
    fn new_cmd_buffer(&self) -> Result<Box<CmdBuffer>>;

    /// Create a `Fence` associated with the command queue.
    fn new_fence(&self) -> Result<handles::Fence>;

    /// Schedule pending command buffers for execution.
    fn flush(&self);
}

/// Trait for command buffers.
///
/// An application can (and should) drop a `CmdBuffer` as soon as it finishes
/// recording commands to the `CmdBuffer` and commiting it.
pub trait CmdBuffer: Object {
    /// Reserve a place for this command buffer on the associated command queue.
    ///
    /// The order in which `enqueue` is called defines the submission order of
    /// command buffers.
    fn enqueue(&mut self) -> Result<()>;

    /// Mark this command buffer as ready for submission.
    fn commit(&mut self) -> Result<()>;

    fn encode_render(
        &mut self,
        render_target_table: &handles::RenderTargetTable,
    ) -> &mut RenderCmdEncoder;
    fn encode_compute(&mut self) -> &mut ComputeCmdEncoder;
    fn encode_copy(&mut self) -> &mut CopyCmdEncoder;

    /// Register a completion handler. Must not be called after calling `commit`.
    fn on_complete(&mut self, cb: Box<FnMut()>);

    /// Wait on a given semaphore before the execution of the command buffer.
    ///
    /// The default implementation panics.
    fn wait_semaphore(&mut self, semaphore: &handles::Semaphore, dst_stage: StageFlags) {
        let _ = (semaphore, dst_stage);
        panic!("Semaphores are not supported by this backend.");
    }

    /// Signal a given semaphore after the execution of the command buffer.
    ///
    /// The default implementation panics.
    fn signal_semaphore(&mut self, semaphore: &handles::Semaphore, src_stage: StageFlags) {
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
    fn host_barrier(
        &mut self,
        src_access: AccessTypeFlags,
        buffers: &[(Range<DeviceSize>, &handles::Buffer)],
    ) {
        let _ = (src_access, buffers);
    }

    /// Acquire resources from another queue with a different queue family.
    ///
    /// This operation is a part of a queue family ownership transfer operation.
    /// See Vulkan 1.0 "6.7.4. Queue Family Ownership Transfer" for details.
    ///
    /// All global memory barriers in `barrier` are ignored. All image/memory
    /// barriers are converted to queue family ownership transfer operations.
    ///
    /// The default implementation panics. Implementations that support more
    /// than one queue families must override this method.
    fn queue_acquire_barrier(&mut self, src_queue_family: QueueFamily, barrier: &handles::Barrier) {
        let _ = (src_queue_family, barrier);
        panic!("Queue families are not supported by this backend.");
    }

    /// Release resources from another queue with a different queue family.
    ///
    /// This operation is a part of a queue family ownership transfer operation.
    /// See Vulkan 1.0 "6.7.4. Queue Family Ownership Transfer" for details.
    ///
    /// All global memory barriers in `barrier` are ignored. All image/memory
    /// barriers are converted to queue family ownership transfer operations.
    ///
    /// The default implementation panics. Implementations that support more
    /// than one queue families must override this method.
    fn queue_release_barrier(&mut self, dst_queue_family: QueueFamily, barrier: &handles::Barrier) {
        let _ = (dst_queue_family, barrier);
        panic!("Queue families are not supported by this backend.");
    }
}

pub trait RenderCmdEncoder: Object + CmdEncoder {
    /// Set the current `RenderPipeline` object.
    ///
    /// All non-dynamic state values of the new `RenderPipeline` will override
    /// the current ones. Other states are left intact.
    fn bind_pipeline(&mut self, pipeline: &handles::RenderPipeline);

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
    /// The current `RenderPipeline` must have been created with rasterization
    /// enabled and `RenderPassRasterizer::set_depth_bias` called with
    /// `Some(Dynamic(_))`.
    fn set_depth_bias(&mut self, value: Option<pipeline::DepthBias>);

    /// Specify the dynamic depth bound values.
    ///
    /// # Valid Usage
    ///
    /// The current `RenderPipeline` must have been created with rasterization
    /// enabled and `RenderPassRasterizer::set_depth_bounds` called with
    /// `Some(Dynamic(_))`.
    ///
    fn set_depth_bounds(&mut self, value: Option<Range<f32>>);

    /// Set the dynamic stencil masks.
    ///
    /// # Valid Usage
    ///
    /// The current `RenderPipeline` must have been created with rasterization
    /// enabled and `RenderPassRasterizer::set_stencil_masks` called with
    /// `Dynamic(_)`.
    ///
    /// `value` must have exactly two elements.
    fn set_stencil_state(&mut self, value: &[pipeline::StencilMasks]);

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
    /// The current `RenderPipeline` must have been created with rasterization
    /// enabled and `RenderPassRasterizer::set_scissors` called with
    /// `Dynamic(_)` for the corresponding viewports.
    fn set_scissors(&mut self, start_viewport: ViewportIndex, value: &[Rect2D<u32>]);

    /// Bind zero or more `ArgTable`s.
    fn bind_arg_table(&mut self, index: ArgTableIndex, tables: &[&handles::ArgTable]);

    /// Bind zero or more vertex buffers.
    fn bind_vertex_buffers(
        &mut self,
        index: VertexBufferIndex,
        buffers: &[(&handles::Buffer, DeviceSize)],
    );

    /// Bind an index buffer.
    fn bind_index_buffer(
        &mut self,
        buffers: &handles::Buffer,
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

pub trait ComputeCmdEncoder: Object + CmdEncoder {
    /// Set the current `ComputePipeline` object.
    fn bind_pipeline(&mut self, pipeline: &handles::ComputePipeline);

    /// Bind zero or more `ArgTable`s.
    fn bind_arg_table(&mut self, index: ArgTableIndex, tables: &[&handles::ArgTable]);

    /// Provoke work in a compute pipeline.
    ///
    /// `workgroup_count` is an array with up to 3 elements.
    fn dispatch(&mut self, workgroup_count: &[u32]);
}

pub trait CopyCmdEncoder: Object + CmdEncoder {
    /// Fill a buffer with a constant byte value.
    ///
    /// Both of `range.start` and `range.end` must be a multiple of 4.
    fn fill_buffer(&mut self, buffer: &handles::Buffer, range: Range<DeviceSize>, value: u8);

    /// Copy data from a buffer to another buffer.
    ///
    /// All of `source_offset`, `destination_offset`, and `size` must be a
    /// multiple of 4.
    fn copy_buffer(
        &mut self,
        src: &handles::Buffer,
        src_offset: DeviceSize,
        dst: &handles::Buffer,
        dst_offset: DeviceSize,
        size: DeviceSize,
    );

    /// Copy data from a buffer to an image.
    ///
    /// The image must be in the `General` or `TransferDestination` layout.
    ///
    /// If the image has a depth/stencil format, the current command queue must
    /// support graphics operations.
    fn copy_buffer_to_image(
        &mut self,
        src: &handles::Buffer,
        src_range: &BufferImageRange,
        dst: &handles::Image,
        dst_layout: resources::ImageLayout,
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
    fn copy_image_to_buffer(
        &mut self,
        src: &handles::Image,
        src_layout: resources::ImageLayout,
        src_aspect: resources::ImageAspect,
        src_range: &resources::ImageLayerRange,
        src_origin: &[u32],
        dst: &handles::Buffer,
        dst_range: &BufferImageRange,
        size: &[u32],
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
        src: &handles::Image,
        src_layout: resources::ImageLayout,
        src_range: &resources::ImageLayerRange,
        src_origin: &[u32],
        dst: &handles::Image,
        dst_layout: resources::ImageLayout,
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
    /// [`begin_debug_group`]: begin_debug_group
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
    /// This ensures the resources are resident at the point of executing the
    /// encoded commands.
    ///
    /// The scope is the current encoder or subpass.
    ///
    /// This method is no-op on `CopyCmdEncoder` since it does not use any
    /// descriptor sets.
    fn use_resource(&mut self, usage: ResourceUsage, objs: &[handles::ResourceRef]);

    /// Declare that the resources in the specified heaps are referenced by the
    /// descriptor sets used on this command encoder.
    ///
    /// This ensures the resources are resident at the point of executing the
    /// encoded commands.
    ///
    /// The scope is the current encoder or subpass.
    ///
    /// This method is no-op on `CopyCmdEncoder` since it does not use any
    /// descriptor sets.
    fn use_heap(&mut self, heaps: &[&heap::Heap]);

    /// Wait on the specified fence and establish an inter-encoder execution
    /// dependency
    ///
    /// The fence must be updated first before waiting on it (according to the
    /// command buffer's submission order). Otherwise, a dead-lock might occur.
    ///
    /// # Valid Usage
    ///
    ///  - `src_stage` must match the `src_state` of the corresponding call to
    ///    `update_fence`.
    ///  - The supported stages of the first access type of each barrier
    ///    defined by `berrier` must be a subset of `src_stage`.
    ///  - You must not wait on a fence that was previously updated in the same
    ///    `CmdEncoder`.
    fn wait_fence(
        &mut self,
        fence: &handles::Fence,
        src_stage: StageFlags,
        barrier: &handles::Barrier,
    );

    /// Update the specified fence.
    ///
    /// A fence can be updated only once. You must create a new one after done
    /// using the old one.
    fn update_fence(&mut self, fence: &handles::Fence, src_stage: StageFlags);

    /// Insert a barrier and establish an execution dependency within the
    /// current encoder or subpass.
    fn barrier(&mut self, barrier: &handles::Barrier);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceUsage {
    Read,
    Write,
    Sample,
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
