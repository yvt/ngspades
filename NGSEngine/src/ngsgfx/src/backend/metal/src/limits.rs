//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;
use cgmath::Vector3;

use std::u32;

#[derive(Debug)]
pub struct DeviceCapabilities {
    limits: core::DeviceLimits,
}

impl DeviceCapabilities {
    pub(crate) fn new(device: metal::MTLDevice) -> Self {
        assert!(!device.is_null());

        let mtptg: metal::MTLSize = device.max_threads_per_threadgroup();

        // https://developer.apple.com/metal/limits/
        // OSX_GPUFamily1_v2
        let limits = core::DeviceLimits {
            supports_heap: false,
            supports_heap_aliasing: false,
            supports_depth_bounds: false,
            supports_cube_array: true,
            max_image_extent_1d: 16384,
            max_image_extent_2d: 16384,
            max_image_extent_3d: 2048,
            max_image_num_array_layers: 2048,
            max_framebuffer_extent: 16384,
            max_compute_workgroup_size: Vector3::new(
                mtptg.width as u32,
                mtptg.height as u32,
                mtptg.depth as u32,
            ),
            max_num_compute_workgroup_invocations: None,
            max_compute_workgroup_count: Vector3::new(
                u32::max_value(),
                u32::max_value(),
                u32::max_value(),
            ),
        };

        Self { limits }
    }
}

impl core::DeviceCapabilities for DeviceCapabilities {
    fn limits(&self) -> &core::DeviceLimits {
        &self.limits
    }
}
