//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `DeviceCaps` for Vulkan, and configurations of the
//! backend.
use ash;
use ash::version::*;
use ash::vk::{self, FALSE};
use bitflags::bitflags;
use flags_macro::flags;
use std::collections::HashMap;
use zangfx_base as base;
use zangfx_base::{zangfx_impl_object, Result};

use crate::formats::{translate_image_format, translate_vertex_format};
use crate::utils::translate_generic_error_unwrap;

/// Properties of a Vulkan physical device as recognized by the ZanGFX Vulkan
/// backend.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub traits: DeviceTraitFlags,
    pub limits: base::DeviceLimits,
    pub queue_families: Vec<base::QueueFamilyInfo>,
    pub memory_types: Vec<base::MemoryTypeInfo>,
    pub memory_regions: Vec<base::MemoryRegionInfo>,
    pub image_features: HashMap<base::ImageFormat, base::ImageFormatCapsFlags>,
    pub vertex_features: HashMap<base::VertexFormat, base::VertexFormatCapsFlags>,
}

bitflags! {
    pub struct DeviceTraitFlags: u8 {
        /// Enables work-arounds for MoltenVK (Vulkan-on-Metal emulation layer).
        const MOLTEN_VK = 0b1;
    }
}

impl DeviceInfo {
    pub fn from_physical_device(
        instance: &ash::Instance,
        phys_device: vk::PhysicalDevice,
        enabled_features: &vk::PhysicalDeviceFeatures,
    ) -> Result<Self> {
        use std::cmp::min;
        let mut traits = flags![DeviceTraitFlags::{}];

        let exts = unsafe {
            instance
                .enumerate_device_extension_properties(phys_device)
                .map_err(translate_generic_error_unwrap)?
        };

        use std::ffi::CStr;
        let mvk_ext_name = CStr::from_bytes_with_nul(b"VK_MVK_moltenvk\0").unwrap();
        let is_molten_vk = exts
            .iter()
            .any(|p| unsafe { CStr::from_ptr(p.extension_name.as_ptr()) } == mvk_ext_name);
        if is_molten_vk {
            traits |= DeviceTraitFlags::MOLTEN_VK;
        }

        let dev_prop = unsafe { instance.get_physical_device_properties(phys_device) };
        let ref dev_limits = dev_prop.limits;
        let limits = base::DeviceLimits {
            supports_heap_aliasing: true,
            supports_depth_bounds: enabled_features.depth_bounds != FALSE,
            supports_cube_array: enabled_features.image_cube_array != FALSE,
            supports_depth_clamp: enabled_features.depth_clamp != FALSE,
            supports_fill_mode_non_solid: enabled_features.fill_mode_non_solid != FALSE,
            max_image_extent_1d: dev_limits.max_image_dimension1_d,
            max_image_extent_2d: dev_limits.max_image_dimension2_d,
            max_image_extent_3d: dev_limits.max_image_dimension3_d,
            max_image_num_array_layers: dev_limits.max_image_array_layers,
            max_render_target_extent: min(
                dev_limits.max_framebuffer_width,
                dev_limits.max_framebuffer_height,
            ),
            max_render_target_num_layers: dev_limits.max_framebuffer_layers,
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
            supports_independent_blend: enabled_features.independent_blend != FALSE,
        };

        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(phys_device) }
                .iter()
                .map(|qf| base::QueueFamilyInfo {
                    caps: translate_queue_flags(qf.queue_flags),
                    count: qf.queue_count as usize,
                })
                .collect();

        let dev_mem = unsafe { instance.get_physical_device_memory_properties(phys_device) };
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
                let fp =
                    unsafe { instance.get_physical_device_format_properties(phys_device, vk_fmt) };
                image_features.insert(
                    fmt,
                    translate_image_format_caps_flags(fp.optimal_tiling_features),
                );
            } else {
                image_features.insert(fmt, flags![base::ImageFormatCapsFlags::{}]);
            }
        }
        for &fmt in base::VertexFormat::values().iter() {
            if let Some(vk_fmt) = translate_vertex_format(fmt) {
                let fp =
                    unsafe { instance.get_physical_device_format_properties(phys_device, vk_fmt) };
                vertex_features.insert(fmt, translate_vertex_format_caps_flags(fp.buffer_features));
            } else {
                vertex_features.insert(fmt, flags![base::VertexFormatCapsFlags::{}]);
            }
        }

        Ok(Self {
            traits,
            limits,
            queue_families,
            image_features,
            vertex_features,
            memory_types,
            memory_regions,
        })
    }
}

fn translate_queue_flags(flags: vk::QueueFlags) -> base::QueueFamilyCapsFlags {
    let mut ret = flags![base::QueueFamilyCapsFlags::{}];
    if flags.intersects(vk::QueueFlags::GRAPHICS) {
        ret |= base::QueueFamilyCapsFlags::RENDER;
        ret |= base::QueueFamilyCapsFlags::COPY;
    }
    if flags.intersects(vk::QueueFlags::COMPUTE) {
        ret |= base::QueueFamilyCapsFlags::COMPUTE;
        ret |= base::QueueFamilyCapsFlags::COPY;
    }
    if flags.intersects(vk::QueueFlags::TRANSFER) {
        ret |= base::QueueFamilyCapsFlags::COPY;
    }
    ret
}

fn translate_memory_type_flags(flags: vk::MemoryPropertyFlags) -> base::MemoryTypeCapsFlags {
    let mut ret = flags![base::MemoryTypeCapsFlags::{}];
    if flags.intersects(vk::MemoryPropertyFlags::DEVICE_LOCAL) {
        ret |= base::MemoryTypeCapsFlags::DEVICE_LOCAL;
    }
    if flags.intersects(vk::MemoryPropertyFlags::HOST_VISIBLE) {
        ret |= base::MemoryTypeCapsFlags::HOST_VISIBLE;
    }
    if flags.intersects(vk::MemoryPropertyFlags::HOST_CACHED) {
        ret |= base::MemoryTypeCapsFlags::HOST_CACHED;
    }
    if flags.intersects(vk::MemoryPropertyFlags::HOST_COHERENT) {
        ret |= base::MemoryTypeCapsFlags::HOST_COHERENT;
    }
    ret
}

fn translate_image_format_caps_flags(value: vk::FormatFeatureFlags) -> base::ImageFormatCapsFlags {
    let mut ret = flags![base::ImageFormatCapsFlags::{}];
    if value.intersects(vk::FormatFeatureFlags::SAMPLED_IMAGE) {
        ret |= base::ImageFormatCapsFlags::SAMPLED;
    }
    if value.intersects(vk::FormatFeatureFlags::STORAGE_IMAGE) {
        ret |= base::ImageFormatCapsFlags::STORAGE;
    }
    if value.intersects(vk::FormatFeatureFlags::STORAGE_IMAGE_ATOMIC) {
        ret |= base::ImageFormatCapsFlags::STORAGE_ATOMIC;
    }
    if value.intersects(vk::FormatFeatureFlags::COLOR_ATTACHMENT) {
        ret |= base::ImageFormatCapsFlags::RENDER;
    }
    if value.intersects(vk::FormatFeatureFlags::COLOR_ATTACHMENT_BLEND) {
        ret |= base::ImageFormatCapsFlags::RENDER_BLEND;
    }
    if value.intersects(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT) {
        ret |= base::ImageFormatCapsFlags::RENDER;
    }
    if value.intersects(vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR) {
        ret |= base::ImageFormatCapsFlags::SAMPLED_FILTER_LINEAR;
    }
    // Without the extension `VK_KHR_maintenance1`, any other flags imply that
    // transfer is possible
    if value.is_empty() {
        // TODO: `FORMAT_FEATURE_TRANSFER_{SRC,DST}_BIT_KHR`
    } else {
        ret |= base::ImageFormatCapsFlags::COPY_READ;
        ret |= base::ImageFormatCapsFlags::COPY_WRITE;
    }
    ret
}

fn translate_vertex_format_caps_flags(
    value: vk::FormatFeatureFlags,
) -> base::VertexFormatCapsFlags {
    let mut ret = flags![base::VertexFormatCapsFlags::{}];
    if value.intersects(vk::FormatFeatureFlags::VERTEX_BUFFER) {
        ret |= base::VertexFormatCapsFlags::VERTEX;
    }
    ret
}

/// Configuration for the ZanGFX Vulkan backend.
#[derive(Debug, Clone, Default)]
pub struct DeviceConfig {
    /// Specifies a set of pairs denoting a queue family index and queue index
    /// allocated for ZanGFX.
    pub queues: Vec<(u32, u32)>,

    /// Optionally specifies a `HeapStrategy` for each memory type.
    pub heap_strategies: Vec<Option<HeapStrategy>>,
}

/// Defines global heaps' memory allocation strategy for a specific memory type.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct HeapStrategy {
    /// The size of "small resource" zones.
    pub small_zone_size: base::DeviceSize,

    /// The size threshold that determines whether a resource should be
    /// allocated in "small resource" zones or not.
    pub size_threshold: base::DeviceSize,
}

impl DeviceConfig {
    /// Construct an empty `DeviceConfig`.
    pub fn new() -> Self {
        Self::default()
    }

    fn validate(&mut self, device_info: &DeviceInfo) {
        for &(qf_index, q_index) in self.queues.iter() {
            if let Some(qf) = device_info.queue_families.get(qf_index as usize) {
                if q_index as usize >= qf.count {
                    panic!("queues: invalid queue index");
                }
            } else {
                panic!("queues: invalid queue family index");
            }
        }

        // Check duplicates
        let queues = self.queues.as_mut_slice();
        queues.sort();
        if queues.iter().zip(queues[1..].iter()).any(|(x, y)| x == y) {
            panic!("queues: duplicate entry");
        }

        // Check the `Vec` of `HeapStrategy`s
        for (i, heap_strategy) in self.heap_strategies.iter().enumerate() {
            if heap_strategy.is_some() && i >= device_info.memory_types.len() {
                panic!("heap_strategies: invalid memory type index");
            }
        }

        self.heap_strategies
            .resize(device_info.memory_types.len(), None);

        for (i, heap_strategy) in self.heap_strategies.iter_mut().enumerate() {
            if heap_strategy.is_none() {
                let region_i = device_info.memory_types[i].region as usize;
                let r_size = device_info.memory_regions[region_i].size;
                *heap_strategy = Some(HeapStrategy::default_with_region_size(r_size))
            }

            // Validate the contents of `HeapStrategy`
            let hs = heap_strategy.unwrap();
            assert!(hs.size_threshold <= hs.small_zone_size);
        }
    }
}

impl HeapStrategy {
    /// Provide a reasonable default value of `HeapStrategy` using the
    /// specified memory region size, based on some heuristics.
    pub fn default_with_region_size(size: base::DeviceSize) -> HeapStrategy {
        assert_ne!(size, 0);
        if size < 65536 {
            Self {
                small_zone_size: 64,
                size_threshold: 0,
            }
        } else if size > 1024u64 * 1024 * 1024 * 4 {
            Self::default_with_region_size(1024u64 * 1024 * 1024 * 4)
        } else {
            Self {
                small_zone_size: size >> 9,
                size_threshold: size >> 11,
            }
        }
    }
}

/// Implementation of `DeviceCaps` for Vulkan
#[derive(Debug)]
pub struct DeviceCaps {
    pub(super) info: DeviceInfo,
    pub(super) config: DeviceConfig,
    available_qfs: Vec<base::QueueFamilyInfo>,
}

zangfx_impl_object! { DeviceCaps: dyn base::DeviceCaps, dyn (crate::Debug) }

impl DeviceCaps {
    /// Construct a `DeviceCaps`. Also perform a validation on the given
    /// `DeviceConfig`.
    pub(super) fn new(info: DeviceInfo, mut config: DeviceConfig) -> Result<Self> {
        // TODO: Consider changing the return type
        config.validate(&info);

        let available_qfs = info
            .queue_families
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
