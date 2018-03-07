//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `DeviceCaps` for Vulkan, and configurations of the
//! backend.
use std::collections::HashMap;
use base;
use ash;
use ash::version::*;
use ash::vk::{self, VK_FALSE};
use common::Result;

use formats::{translate_image_format, translate_vertex_format};

/// Properties of a Vulkan physical device as recognized by the ZanGFX Vulkan
/// backend.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub limits: base::DeviceLimits,
    pub queue_families: Vec<base::QueueFamilyInfo>,
    pub memory_types: Vec<base::MemoryTypeInfo>,
    pub memory_regions: Vec<base::MemoryRegionInfo>,
    pub image_features: HashMap<base::ImageFormat, base::ImageFormatCapsFlags>,
    pub vertex_features: HashMap<base::VertexFormat, base::VertexFormatCapsFlags>,
}

impl DeviceInfo {
    pub fn from_physical_device(
        instance: &ash::Instance<V1_0>,
        phys_device: vk::PhysicalDevice,
        enabled_features: &vk::PhysicalDeviceFeatures,
    ) -> Self {
        use std::cmp::min;
        let dev_prop = instance.get_physical_device_properties(phys_device);
        let ref dev_limits = dev_prop.limits;
        let limits = base::DeviceLimits {
            supports_heap_aliasing: true,
            supports_depth_bounds: enabled_features.depth_bounds != VK_FALSE,
            supports_cube_array: enabled_features.image_cube_array != VK_FALSE,
            supports_depth_clamp: enabled_features.depth_clamp != VK_FALSE,
            supports_fill_mode_non_solid: enabled_features.fill_mode_non_solid != VK_FALSE,
            max_image_extent_1d: dev_limits.max_image_dimension1d,
            max_image_extent_2d: dev_limits.max_image_dimension2d,
            max_image_extent_3d: dev_limits.max_image_dimension3d,
            max_image_num_array_layers: dev_limits.max_image_array_layers,
            max_render_target_extent: min(
                dev_limits.max_framebuffer_width,
                dev_limits.max_framebuffer_height,
            ),
            max_compute_workgroup_size: [
                dev_limits.max_compute_work_group_size[0],
                dev_limits.max_compute_work_group_size[1],
                dev_limits.max_compute_work_group_size[2],
            ],
            max_num_compute_workgroup_invocations: dev_limits.max_compute_work_group_invocations,
            max_compute_workgroup_count: [
                dev_limits.max_compute_work_group_count[0],
                dev_limits.max_compute_work_group_count[1],
                dev_limits.max_compute_work_group_count[2],
            ],
            max_num_viewports: dev_limits.max_viewports,
            uniform_buffer_align: dev_limits.min_uniform_buffer_offset_alignment as _,
            storage_buffer_align: dev_limits.min_storage_buffer_offset_alignment as _,
            supports_semaphore: true,
            supports_independent_blend: enabled_features.independent_blend != VK_FALSE,
        };

        let queue_families = instance
            .get_physical_device_queue_family_properties(phys_device)
            .iter()
            .map(|qf| base::QueueFamilyInfo {
                caps: translate_queue_flags(qf.queue_flags),
                count: qf.queue_count as usize,
            })
            .collect();

        let dev_mem = instance.get_physical_device_memory_properties(phys_device);
        let memory_types = dev_mem.memory_types[0..dev_mem.memory_type_count as usize]
            .iter()
            .map(|mt| base::MemoryTypeInfo {
                caps: translate_memory_type_flags(mt.property_flags),
                region: mt.heap_index,
            })
            .collect();
        let memory_regions = dev_mem.memory_heaps[0..dev_mem.memory_heap_count as usize]
            .iter()
            .map(|mh| base::MemoryRegionInfo { size: mh.size })
            .collect();

        let mut image_features = HashMap::new();
        let mut vertex_features = HashMap::new();

        for &fmt in base::ImageFormat::values().iter() {
            if let Some(vk_fmt) = translate_image_format(fmt) {
                let fp = instance.get_physical_device_format_properties(phys_device, vk_fmt);
                image_features.insert(
                    fmt,
                    translate_image_format_caps_flags(fp.optimal_tiling_features),
                );
            } else {
                image_features.insert(fmt, flags![base::ImageFormatCaps::{}]);
            }
        }
        for &fmt in base::VertexFormat::values().iter() {
            if let Some(vk_fmt) = translate_vertex_format(fmt) {
                let fp = instance.get_physical_device_format_properties(phys_device, vk_fmt);
                vertex_features.insert(fmt, translate_vertex_format_caps_flags(fp.buffer_features));
            } else {
                vertex_features.insert(fmt, flags![base::VertexFormatCaps::{}]);
            }
        }

        Self {
            limits,
            queue_families,
            image_features,
            vertex_features,
            memory_types,
            memory_regions,
        }
    }
}

fn translate_queue_flags(flags: vk::QueueFlags) -> base::QueueFamilyCapsFlags {
    let mut ret = flags![base::QueueFamilyCaps::{}];
    if flags.intersects(vk::QUEUE_GRAPHICS_BIT) {
        ret |= base::QueueFamilyCaps::Render;
        ret |= base::QueueFamilyCaps::Copy;
    }
    if flags.intersects(vk::QUEUE_COMPUTE_BIT) {
        ret |= base::QueueFamilyCaps::Compute;
        ret |= base::QueueFamilyCaps::Copy;
    }
    if flags.intersects(vk::QUEUE_TRANSFER_BIT) {
        ret |= base::QueueFamilyCaps::Copy;
    }
    ret
}

fn translate_memory_type_flags(flags: vk::MemoryPropertyFlags) -> base::MemoryTypeCapsFlags {
    let mut ret = flags![base::MemoryTypeCaps::{}];
    if flags.intersects(vk::MEMORY_PROPERTY_DEVICE_LOCAL_BIT) {
        ret |= base::MemoryTypeCaps::DeviceLocal;
    }
    if flags.intersects(vk::MEMORY_PROPERTY_HOST_VISIBLE_BIT) {
        ret |= base::MemoryTypeCaps::HostVisible;
    }
    if flags.intersects(vk::MEMORY_PROPERTY_HOST_CACHED_BIT) {
        ret |= base::MemoryTypeCaps::HostCached;
    }
    if flags.intersects(vk::MEMORY_PROPERTY_HOST_COHERENT_BIT) {
        ret |= base::MemoryTypeCaps::HostCoherent;
    }
    ret
}

fn translate_image_format_caps_flags(value: vk::FormatFeatureFlags) -> base::ImageFormatCapsFlags {
    let mut ret = flags![base::ImageFormatCaps::{}];
    if value.intersects(vk::FORMAT_FEATURE_SAMPLED_IMAGE_BIT) {
        ret |= base::ImageFormatCaps::Sampled;
    }
    if value.intersects(vk::FORMAT_FEATURE_STORAGE_IMAGE_BIT) {
        ret |= base::ImageFormatCaps::Storage;
    }
    if value.intersects(vk::FORMAT_FEATURE_STORAGE_IMAGE_ATOMIC_BIT) {
        ret |= base::ImageFormatCaps::StorageAtomic;
    }
    if value.intersects(vk::FORMAT_FEATURE_COLOR_ATTACHMENT_BIT) {
        ret |= base::ImageFormatCaps::Render;
    }
    if value.intersects(vk::FORMAT_FEATURE_COLOR_ATTACHMENT_BLEND_BIT) {
        ret |= base::ImageFormatCaps::RenderBlend;
    }
    if value.intersects(vk::FORMAT_FEATURE_DEPTH_STENCIL_ATTACHMENT_BIT) {
        ret |= base::ImageFormatCaps::Render;
    }
    if value.intersects(vk::FORMAT_FEATURE_SAMPLED_IMAGE_FILTER_LINEAR_BIT) {
        ret |= base::ImageFormatCaps::SampledFilterLinear;
    }
    // Without the extension `VK_KHR_maintenance1`, any other flags imply that
    // transfer is possible
    if value.is_empty() {
        // TODO: `FORMAT_FEATURE_TRANSFER_{SRC,DST}_BIT_KHR`
    } else {
        ret |= base::ImageFormatCaps::CopyRead;
        ret |= base::ImageFormatCaps::CopyWrite;
    }
    ret
}

fn translate_vertex_format_caps_flags(
    value: vk::FormatFeatureFlags,
) -> base::VertexFormatCapsFlags {
    let mut ret = flags![base::VertexFormatCaps::{}];
    if value.intersects(vk::FORMAT_FEATURE_VERTEX_BUFFER_BIT) {
        ret |= base::VertexFormatCaps::Vertex;
    }
    ret
}

/// Configuration for the ZanGFX Vulkan backend.
#[derive(Debug, Clone, Default)]
pub struct DeviceConfig {
    /// Specifies a set of pairs denoting a queue family index and queue index
    /// allocated for ZanGFX.
    pub queues: Vec<(u32, u32)>,
}

impl DeviceConfig {
    /// Construct an empty `DeviceConfig`.
    pub fn new() -> Self {
        Self::default()
    }

    fn validate(&mut self, device_info: &DeviceInfo) -> Result<()> {
        use common::{Error, ErrorKind};

        for &(qf_index, q_index) in self.queues.iter() {
            if let Some(qf) = device_info.queue_families.get(qf_index as usize) {
                if q_index as usize >= qf.count {
                    return Err(Error::with_detail(
                        ErrorKind::InvalidUsage,
                        "queues: invalid queue index",
                    ));
                }
            } else {
                return Err(Error::with_detail(
                    ErrorKind::InvalidUsage,
                    "queues: invalid queue family index",
                ));
            }
        }

        // Check duplicates
        let queues = self.queues.as_mut_slice();
        queues.sort();
        if queues.iter().zip(queues[1..].iter()).any(|(x, y)| x == y) {
            return Err(Error::with_detail(
                ErrorKind::InvalidUsage,
                "queues: duplicate entry",
            ));
        }

        Ok(())
    }
}

/// Implementation of `DeviceCaps` for Vulkan
#[derive(Debug)]
pub struct DeviceCaps {
    pub(super) info: DeviceInfo,
    config: DeviceConfig,
    available_qfs: Vec<base::QueueFamilyInfo>,
}

zangfx_impl_object! { DeviceCaps: base::DeviceCaps, ::Debug }

impl DeviceCaps {
    /// Construct a `DeviceCaps`. Also perform a validation on the given
    /// `DeviceConfig`.
    pub(super) fn new(info: DeviceInfo, mut config: DeviceConfig) -> Result<Self> {
        config.validate(&info)?;

        let available_qfs = info.queue_families
            .iter()
            .enumerate()
            .map(|(qf_i, qf)| base::QueueFamilyInfo {
                caps: qf.caps,
                count: config
                    .queues
                    .iter()
                    .filter(|&&(i, _)| i == qf_i as u32)
                    .count(),
            })
            .collect();

        Ok(Self {
            info,
            config,
            available_qfs,
        })
    }
}

impl base::DeviceCaps for DeviceCaps {
    fn limits(&self) -> &base::DeviceLimits {
        &self.info.limits
    }

    fn image_format_caps(&self, format: base::ImageFormat) -> base::ImageFormatCapsFlags {
        *self.info.image_features.get(&format).unwrap()
    }

    fn vertex_format_caps(&self, format: base::VertexFormat) -> base::VertexFormatCapsFlags {
        *self.info.vertex_features.get(&format).unwrap()
    }

    fn memory_types(&self) -> &[base::MemoryTypeInfo] {
        &self.info.memory_types
    }

    fn memory_regions(&self) -> &[base::MemoryRegionInfo] {
        &self.info.memory_regions
    }

    fn queue_families(&self) -> &[base::QueueFamilyInfo] {
        &self.available_qfs
    }
}
