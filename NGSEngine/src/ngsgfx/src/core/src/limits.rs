//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::fmt::Debug;

use cgmath::Vector3;

#[derive(Debug, Clone, Copy)]
pub struct DeviceLimits {
    /// Indicates whether the backend supports the memory managment using manually-allocated heaps or not.
    ///
    /// If this is `false`,
    /// - textures and buffers are allocated from an API-managed global heap and
    ///   aliasing is not supported (implies `supports_heap_aliasing == false`).
    /// - The value of `HeapDescription::size` is ignored.
    /// - `Factory::get_buffer_memory_requirements` and `Factory::get_image_memory_requirements` will return
    ///   dummy values.
    pub supports_heap: bool,

    /// Indicates whether `MappableHeap::make_aliasable` is supported or not.
    pub supports_heap_aliasing: bool,

    pub supports_depth_bounds: bool,

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
