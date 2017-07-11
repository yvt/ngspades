//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::fmt::Debug;

use cgmath::Vector3;

#[derive(Debug, Clone, Copy)]
pub struct DeviceLimits {
    /// Indicates whether the backend supports the memory management using
    /// specialized heaps or not.
    ///
    /// If this is `false`,
    /// - textures and buffers are allocated from an API-managed global heap and
    ///   aliasing is not supported (implies `supports_heap_aliasing == false`).
    /// - The value of `HeapDescription::size` is ignored.
    /// - `Factory::get_buffer_memory_requirements` and `Factory::get_image_memory_requirements` will return
    ///   dummy values.
    /// - `Factory::make_specialized_heap` succeeds, but the returned heap might
    ///   might behave like, i.e. exhibits similar performance characteristics to
    ///   those of universal heaps and never runs out of an internal space.
    pub supports_specialized_heap: bool,

    /// Indicates whether `MappableHeap::make_aliasable` is supported or not.
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

pub trait DeviceCapabilities: Debug + Send + Sync {
    fn limits(&self) -> &DeviceLimits;
}
