//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::any::Any;
use std::fmt::Debug;
use cgmath::Vector3;
use ngsenumflags::BitFlags;

use formats::{ImageFormat, VertexFormat};

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

#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum VertexFormatCaps {
    Vertex = 0b1,
}

pub type ImageFormatCapsFlags = BitFlags<ImageFormatCaps>;
pub type VertexFormatCapsFlags = BitFlags<VertexFormatCaps>;

pub trait DeviceCaps: Send + Sync + Any + Debug + AsRef<Any> + AsMut<Any> {
    fn limits(&self) -> &DeviceLimits;
    fn image_format_caps(&self, format: ImageFormat) -> ImageFormatCapsFlags;
    fn vertex_format_caps(&self, format: VertexFormat) -> VertexFormatCapsFlags;
}
