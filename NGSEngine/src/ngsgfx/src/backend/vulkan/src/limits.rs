//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, ash};
use cgmath::Vector3;
use ash::version::{V1_0, InstanceV1_0};
use ash::vk::types::{PhysicalDevice, PhysicalDeviceMemoryProperties, PhysicalDeviceProperties,
                     PhysicalDeviceFeatures, VK_FALSE, PhysicalDeviceLimits, MemoryType};
use ngsgfx_common::int::BinaryInteger;
use std::u32;

/// The maximum number of internal queues.
///
/// This value is guaranteed to be less than 32 and greater than 0. This is mainly
/// because the implementation of this backend often uses `u32` bit fields to
/// represent sets of internal queues.
pub const MAX_NUM_QUEUES: usize = 4;

#[derive(Debug, Hash, Clone)]
pub struct DeviceConfig {
    /// Specifies the queue family index and queue index for each internal queue
    /// to be created.
    ///
    /// The number of elements must be less than or equal to `MAX_NUM_QUEUES`.
    pub queues: Vec<(u32, u32)>,

    pub engine_queue_mappings: EngineQueueMappings,

    /// Specifies mappings from `StorageMode` to memory types.
    pub storage_mode_mappings: StorageModeMappings,

    pub memory_types: Vec<(MemoryType, HeapStrategy)>,
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

    pub fn internal_queues_for_engines(&self, index: core::DeviceEngineFlags) -> u32 {
        let mut bits = 0u32;
        if index.contains(core::DeviceEngine::Universal) {
            bits.set_bit(self.universal as u32);
        }
        if index.contains(core::DeviceEngine::Compute) {
            bits.set_bit(self.compute as u32);
        }
        if index.contains(core::DeviceEngine::Copy) {
            bits.set_bit(self.copy as u32);
        }
        bits
    }
}

/// Defines `UniversalHeap`'s memory allocation strategy for a specific memory type.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct HeapStrategy {
    /// The size of "small resource" zones.
    pub small_zone_size: core::DeviceSize,

    /// The size threshold that determines whether a resource should be
    /// allocated in "small resource" zones or not.
    pub size_threshold: core::DeviceSize,
}

impl HeapStrategy {
    /// Provide a reasonable default value of `HeapStrategy` using the
    /// specified heap size, based on some heuristics.
    pub fn default_with_heap_size(size: core::DeviceSize) -> HeapStrategy {
        assert_ne!(size, 0);
        if size < 65536 {
            Self {
                small_zone_size: 64,
                size_threshold: 0,
            }
        } else if size > 1024u64 * 1024 * 1024 * 4 {
            Self::default_with_heap_size(1024u64 * 1024 * 1024 * 4)
        } else {
            Self {
                small_zone_size: size >> 9,
                size_threshold: size >> 11,
            }
        }
    }
}

/// Defines mapping from `StorageMode` to memory types.
///
/// Each field contains a list of memory types. During a resource allocation,
/// each item (from first to last) is checked against the memory requirements
/// and the first matching item is selected.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct StorageModeMappings {
    /// Memory type candidates for `StorageMode::Private`.
    ///
    /// Must have at least one element.
    pub private: Vec<u8>,

    /// Memory type candidates for `StorageMode::Shared`.
    ///
    /// Must have at least one element.
    pub shared: Vec<u8>,

    /// Memory type candidates for `StorageMode::Memoryless`.
    pub memoryless: Vec<u8>,
}

impl StorageModeMappings {
    pub fn memory_types_for_storage_mode(&self, index: core::StorageMode) -> &[u8] {
        match index {
            core::StorageMode::Private => &self.private,
            core::StorageMode::Shared => &self.shared,
            core::StorageMode::Memoryless => &self.memoryless,
        }
    }

    pub fn map_storage_mode(&self, storage_mode: core::StorageMode, valid_bits: u32) -> Option<u8> {
        use ngsgfx_common::int::BinaryInteger;

        self.memory_types_for_storage_mode(storage_mode)
            .iter()
            .filter(|&&t| valid_bits.get_bit(t as u32))
            .nth(0)
            .cloned()
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
