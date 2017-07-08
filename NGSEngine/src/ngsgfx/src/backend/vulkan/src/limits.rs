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

#[derive(Debug)]
pub struct DeviceCapabilities {
    limits: core::DeviceLimits,
    pub(crate) mem_prop: PhysicalDeviceMemoryProperties,
    pub(crate) dev_prop: PhysicalDeviceProperties,
    pub(crate) qf_props: Vec<QueueFamilyProperties>,
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
        let qf_props: Vec<QueueFamilyProperties> =
            instance.get_physical_device_queue_family_properties(phys_device);
        let limits;

        {
            let ref dev_limits: PhysicalDeviceLimits = dev_prop.limits;
            limits = core::DeviceLimits {
                supports_specialized_heap: true,
                supports_heap_aliasing: true,
                supports_depth_bounds: enabled_features.depth_bounds != VK_FALSE,
                supports_cube_array: enabled_features.image_cube_array != VK_FALSE,
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
            qf_props,
        }
    }
}

impl core::DeviceCapabilities for DeviceCapabilities {
    fn limits(&self) -> &core::DeviceLimits {
        &self.limits
    }
}
