//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, ash};
use cgmath::Vector3;
use ash::version::{V1_0, InstanceV1_0};
use ash::vk::types::{PhysicalDevice, PhysicalDeviceMemoryProperties, PhysicalDeviceProperties,
                     QueueFamilyProperties, PhysicalDeviceFeatures, VK_FALSE, PhysicalDeviceLimits};

use std::u32;
use std::ops;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct DeviceConfig {
    /// Specifies the queue family index and queue index for each internal queue
    /// to be created.
    ///
    /// The number of elements must be less than or equal to 32.
    pub queues: Vec<(u32, u32)>,

    pub engine_queue_mappings: EngineQueueMappings,
}

/// Defines mappings from `DeviceEngine`s to internal queue indices.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct EngineQueueMappings {
    pub universal: usize,
    pub compute: usize,
    pub copy: usize,
}

impl EngineQueueMappings {
    pub fn internal_queue_for_engine(&self, index: core::DeviceEngine) -> Option<usize> {
        match index {
            core::DeviceEngine::Universal => Some(self.universal),
            core::DeviceEngine::Compute => Some(self.compute),
            core::DeviceEngine::Copy => Some(self.copy),
            core::DeviceEngine::Host => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    limits: core::DeviceLimits,
    pub(crate) mem_prop: PhysicalDeviceMemoryProperties,
    pub(crate) dev_prop: PhysicalDeviceProperties,
}

impl DeviceCapabilities {
    pub(crate) fn new(
        instance: &ash::Instance<V1_0>,
        phys_device: PhysicalDevice,
        enabled_features: &PhysicalDeviceFeatures,
    ) -> Self {
        let mem_prop: PhysicalDeviceMemoryProperties =
            instance.get_physical_device_memory_properties(phys_device);
        let dev_prop: PhysicalDeviceProperties =
            instance.get_physical_device_properties(phys_device);
        let limits;

        {
            let ref dev_limits: PhysicalDeviceLimits = dev_prop.limits;
            limits = core::DeviceLimits {
                supports_specialized_heap: true,
                supports_heap_aliasing: true,
                supports_depth_bounds: enabled_features.depth_bounds != VK_FALSE,
                supports_cube_array: enabled_features.image_cube_array != VK_FALSE,
                supports_depth_clamp: enabled_features.depth_clamp != VK_FALSE,
                supports_fill_mode_non_solid: enabled_features.fill_mode_non_solid != VK_FALSE,
                max_image_extent_1d: dev_limits.max_image_dimension1d,
                max_image_extent_2d: dev_limits.max_image_dimension2d,
                max_image_extent_3d: dev_limits.max_image_dimension3d,
                max_image_num_array_layers: dev_limits.max_image_array_layers,
                max_framebuffer_extent: *[
                    dev_limits.max_framebuffer_width,
                    dev_limits.max_framebuffer_height,
                ].iter()
                    .min()
                    .unwrap(),
                max_compute_workgroup_size: Vector3::new(
                    dev_limits.max_compute_work_group_size[0],
                    dev_limits.max_compute_work_group_size[1],
                    dev_limits.max_compute_work_group_size[2],
                ),
                max_num_compute_workgroup_invocations: dev_limits
                    .max_compute_work_group_invocations,
                max_compute_workgroup_count: Vector3::new(
                    dev_limits.max_compute_work_group_count[0],
                    dev_limits.max_compute_work_group_count[1],
                    dev_limits.max_compute_work_group_count[2],
                ),
            };
        }

        Self {
            limits,
            mem_prop,
            dev_prop,
        }
    }
}

impl core::DeviceCapabilities for DeviceCapabilities {
    fn limits(&self) -> &core::DeviceLimits {
        &self.limits
    }
}
