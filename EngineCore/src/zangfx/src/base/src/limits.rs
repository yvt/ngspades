//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides an information regarding a device's capabilities and limits.
use bitflags::bitflags;

use crate::formats::{ImageFormat, VertexFormat};
use crate::Object;
use crate::{DeviceSize, MemoryRegionIndex};

#[derive(Debug, Clone, Copy)]
pub struct DeviceLimits {
    /// Indicates whether [`Heap::make_aliasable`] is supported or not.
    ///
    /// [`Heap::make_aliasable`]: crate::Heap::make_aliasable
    pub supports_heap_aliasing: bool,

    /// Indicates whether *creating* semaphores (inter-queue synchronization) are
    /// supported or not.
    pub supports_semaphore: bool,

    /// Indicates whether `GraphicsPipelineRasterizerDescription::depth_bounds`
    /// can have values other than `None`.
    pub supports_depth_bounds: bool,

    /// Indicates whether `GraphicsPipelineRasterizerDescription::depth_clip_mode`
    /// can have a value of `DepthClipMode::Clamp`.
    pub supports_depth_clamp: bool,

    /// Indicates whether `GraphicsPipelineRasterizerDescription::triangle_fill_mode`
    /// can have a value of `TriangleFillMode::Line`.
    pub supports_fill_mode_non_solid: bool,

    pub supports_cube_array: bool,

    pub supports_independent_blend: bool,

    pub max_image_extent_1d: u32,
    pub max_image_extent_2d: u32,
    pub max_image_extent_3d: u32,
    pub max_image_num_array_layers: u32,
    pub max_render_target_extent: u32,
    pub max_render_target_num_layers: u32,

    pub max_num_viewports: u32,

    /// Indicates the maximum size of a local compute workgroup (specified by
    /// the `LocalSize` execution mode and by the object decorated by the
    /// `WorkgroupSize` decoration in a SPIR-V shader module).
    pub max_compute_workgroup_size: [u32; 3],

    /// Indicates the maximum total number of compute shader invocations in a
    /// single local compute workgroup.
    pub max_num_compute_workgroup_invocations: u32,

    /// Indicates the maximum number of compute local workgroups.
    pub max_compute_workgroup_count: [u32; 3],

    /// The minimum alignment requirement for uniform buffers, measured in
    /// bytes.
    ///
    /// Must be equal to or less than 256 bytes.
    pub uniform_buffer_align: DeviceSize,

    /// The minimum alignment requirement for storage buffers, measured in
    /// bytes.
    ///
    /// Must be equal to or less than 256 bytes.
    pub storage_buffer_align: DeviceSize,
    // TODO: expose more limits
}

bitflags! {
    /// Indicates a set of operations on a specific `ImageFormat` supported by
    /// a device.
    pub struct ImageFormatCapsFlags: u16 {
        const Sampled = 0b000000001;
        const SampledFilterLinear = 0b000000010;
        const Storage = 0b000000100;
        const StorageAtomic = 0b000001000;
        const Render = 0b000010000;
        const RenderBlend = 0b000100000;
        const CopyRead = 0b010000000;
        const CopyWrite = 0b100000000;
    }
}

bitflags! {
    /// Indicates a set of operations on a specific `VertexFormat` supported by a
    /// device.
    pub struct VertexFormatCapsFlags: u8 {
        const Vertex = 0b1;
    }
}

bitflags! {
    /// Indicates a capability of a specific memory type of a device.
    ///
    /// See Vulkan 1.0 Specification "10.2. Device Memory" for details and usage.
    pub struct MemoryTypeCapsFlags: u8 {
        const HostVisible = 0b0001;
        /// Indicates that the coherency of the memory contents between the host and
        /// the device is maintained automatically. Note that even with this flag
        /// you still have to insert appropriate memory barriers by issuing
        /// [`host_barrier`] commands.
        ///
        /// For a memory type without this flag, you must perform cache maintenance
        /// operations manually. (Currently API does not define a way to do this.
        /// Therefore, host-visible memory types without this flag are practially
        /// useless.)
        ///
        /// [`host_barrier`]: crate::CmdBuffer::host_barrier
        const HostCoherent = 0b0010;
        const HostCached = 0b0100;
        const DeviceLocal = 0b1000;
    }
}

/// Describes the properties of a specific memory type of a device.
#[derive(Debug, Clone, Copy)]
pub struct MemoryTypeInfo {
    pub caps: MemoryTypeCapsFlags,
    pub region: MemoryRegionIndex,
}

/// Describes the properties of a specific memory region of a device.
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegionInfo {
    pub size: DeviceSize,
}

bitflags! {
    /// Indicates a capability of a specific queue family of a device.
    ///
    /// See Vulkan 1.0 Specification "4.1. Physical Devices" for details and usage.
    pub struct QueueFamilyCapsFlags: u8 {
        const Render = 0b001;
        const Compute = 0b010;
        const Copy = 0b100;
    }
}

/// Describes the properties of a specific queue family of a device.
#[derive(Debug, Clone, Copy)]
pub struct QueueFamilyInfo {
    pub caps: QueueFamilyCapsFlags,
    pub count: usize,
}

/// Describes the properties and capabilities of a device.
pub trait DeviceCaps: Object {
    /// Return the implementation limits of the device.
    fn limits(&self) -> &DeviceLimits;

    /// Return the device capabilies on a given image format.
    fn image_format_caps(&self, format: ImageFormat) -> ImageFormatCapsFlags;

    /// Return the device capabilies on a given vertex format.
    fn vertex_format_caps(&self, format: VertexFormat) -> VertexFormatCapsFlags;

    /// Return the memory types provided by the device.
    ///
    /// The ordering must follow that of Vulkan's
    /// `VkPhysicalDeviceMemoryProperties`. See Vulkan 1.0 "10.2. Device Memory"
    /// for details.
    fn memory_types(&self) -> &[MemoryTypeInfo];

    /// Return the memory regions provided by the device.
    fn memory_regions(&self) -> &[MemoryRegionInfo];

    /// Return the queue families provided by the device.
    fn queue_families(&self) -> &[QueueFamilyInfo];
}
