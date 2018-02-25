//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides an information regarding a device's capabilities and limits.
use std::any::Any;
use std::fmt::Debug;
use cgmath::Vector3;
use ngsenumflags::BitFlags;

use formats::{ImageFormat, VertexFormat};
use {DeviceSize, MemoryRegionIndex};

#[derive(Debug, Clone, Copy)]
pub struct DeviceLimits {
    // TODO: port `DeviceLimits` to ZanGFX
    /// Indicates whether [`Heap::make_aliasable`] is supported or not.
    ///
    /// [`Heap::make_aliasable`]: make_aliasable
    pub supports_heap_aliasing: bool,

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

    pub max_image_extent_1d: u32,
    pub max_image_extent_2d: u32,
    pub max_image_extent_3d: u32,
    pub max_image_num_array_layers: u32,
    pub max_framebuffer_extent: u32,

    /// Indicates the maximum size of a local compute workgroup (specified by
    /// the `LocalSize` execution mode and by the object decorated by the
    /// `WorkgroupSize` decoration in a SPIR-V shader module).
    pub max_compute_workgroup_size: Vector3<u32>,

    /// Indicates the maximum total number of compute shader invocations in a
    /// single local compute workgroup.
    pub max_num_compute_workgroup_invocations: u32,

    /// Indicates the maximum number of compute local workgroups.
    pub max_compute_workgroup_count: Vector3<u32>,
    // TODO: expose more limits
}

/// Indicates an operation on a specific `ImageFormat` supported by a device.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageFormatCaps {
    Sampled = 0b000000001,
    SampledFilterLinear = 0b000000010,
    Storage = 0b000000100,
    StorageAtomic = 0b000001000,
    Render = 0b000010000,
    RenderBlend = 0b000100000,
    CopyRead = 0b010000000,
    CopyWrite = 0b100000000,
}

/// Indicates an operation on a specific `VertexFormat` supported by a device.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum VertexFormatCaps {
    Vertex = 0b1,
}

/// Indicates a set of operations on a specific `ImageFormat` supported by a
/// device.
pub type ImageFormatCapsFlags = BitFlags<ImageFormatCaps>;

/// Indicates a set of operations on a specific `VertexFormat` supported by a
/// device.
pub type VertexFormatCapsFlags = BitFlags<VertexFormatCaps>;

/// Indicates a capability of a specific memory type of a device.
///
/// See Vulkan 1.0 Specification "10.2. Device Memory" for details and usage.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryTypeCaps {
    HostVisible = 0b0001,
    HostCoherent = 0b0010,
    HostCached = 0b0100,
    DeviceLocal = 0b1000,
}

/// Indicates a set of capabilities of a specific memory type of a device.
pub type MemoryTypeCapsFlags = BitFlags<MemoryTypeCaps>;

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

/// Indicates a capability of a specific queue family of a device.
///
/// See Vulkan 1.0 Specification "4.1. Physical Devices" for details and usage.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum QueueFamilyCaps {
    Render = 0b001,
    Compute = 0b010,
    Copy = 0b100,
}

/// Indicates a set of capabilities of a specific queue family of a device.
pub type QueueFamilyCapsFlags = BitFlags<QueueFamilyCaps>;

/// Describes the properties of a specific queue family of a device.
#[derive(Debug, Clone, Copy)]
pub struct QueueFamilyInfo {
    pub caps: QueueFamilyCapsFlags,
    pub count: usize,
}

/// Describes the properties and capabilities of a device.
pub trait DeviceCaps: Send + Sync + Any + Debug + AsRef<Any> + AsMut<Any> {
    fn limits(&self) -> &DeviceLimits;
    fn image_format_caps(&self, format: ImageFormat) -> ImageFormatCapsFlags;
    fn vertex_format_caps(&self, format: VertexFormat) -> VertexFormatCapsFlags;
    fn memory_types(&self) -> &[MemoryTypeInfo];
    fn memory_regions(&self) -> &[MemoryRegionInfo];
    fn queue_families(&self) -> &[QueueFamilyInfo];
}
