//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ngsenumflags::flags;

use zangfx_base::{self as base, Result};
use zangfx_common::BinaryInteger;

/// An extension trait for `Device`.
pub trait DeviceUtils {
    /// Find the optimal memory type specific to the device based on the
    /// requirements.
    ///
    /// - `valid_memory_types` represents a set of memory types valid for your
    ///   intended usage.
    /// - `optimal_caps` represents the capabilities of the memory type
    ///   preferred by the application.
    /// - `required_caps` represents the capabilities the memory type must have.
    ///
    fn choose_memory_type(
        &self,
        valid_memory_types: u32,
        optimal_caps: base::MemoryTypeCapsFlags,
        required_caps: base::MemoryTypeCapsFlags,
    ) -> Option<base::MemoryType>;

    /// Get the supported memory types for the given buffer usage.
    fn memory_types_for_buffer(&self, usage: base::BufferUsageFlags) -> Result<u32>;

    /// Get the supported memory types for the given image format.
    ///
    /// The returned values are guaranteed to be identical for all color
    /// formats. This, however, does not apply to depth/stencil formats.
    /// The backend implementation should ensure that as per Vulkan 1.0
    /// "11.6. Resource Memory Association".
    fn memory_types_for_image(&self, format: base::ImageFormat) -> Result<u32>;

    /// Find the optimal memory type for the given buffer usage.
    ///
    /// Unless any error occurs during the process, this method is guaranteed to
    /// return some memory type provided that `required_caps` is a subset of at
    /// least one of the following: `DeviceLocal` and
    /// `HostVisible | HostCoherent`.
    fn memory_type_for_buffer(
        &self,
        usage: base::BufferUsageFlags,
        optimal_caps: base::MemoryTypeCapsFlags,
        required_caps: base::MemoryTypeCapsFlags,
    ) -> Result<Option<base::MemoryType>>;

    /// Find the optimal memory type for the given image format.
    ///
    /// Unless any error occurs during the process, this method is guaranteed to
    /// return some memory type provided that `required_caps` is a subset of at
    /// least one of the following: `DeviceLocal`.
    ///
    /// Note: Images are never host-visible in ZanGFX.
    fn memory_type_for_image(
        &self,
        format: base::ImageFormat,
        optimal_caps: base::MemoryTypeCapsFlags,
        required_caps: base::MemoryTypeCapsFlags,
    ) -> Result<Option<base::MemoryType>>;
}

impl DeviceUtils for dyn base::Device {
    fn choose_memory_type(
        &self,
        valid_memory_types: u32,
        optimal_caps: base::MemoryTypeCapsFlags,
        required_caps: base::MemoryTypeCapsFlags,
    ) -> Option<base::MemoryType> {
        // Based on the algorithm shown in Vulkan specification 1.0
        // "10.2. Device Memory".
        let memory_types = self.caps().memory_types();

        for i in valid_memory_types.one_digits() {
            if memory_types[i as usize].caps.contains(optimal_caps) {
                return Some(i);
            }
        }

        for i in valid_memory_types.one_digits() {
            if memory_types[i as usize].caps.contains(required_caps) {
                return Some(i);
            }
        }

        None
    }

    fn memory_types_for_buffer(&self, usage: base::BufferUsageFlags) -> Result<u32> {
        let buffer = self.build_buffer().size(1).usage(usage).build()?;
        Ok(buffer.get_memory_req()?.memory_types)
    }

    fn memory_types_for_image(&self, format: base::ImageFormat) -> Result<u32> {
        let image = self
            .build_image()
            .extents(&[1, 1])
            .usage(flags![base::ImageUsage::{CopyRead | CopyWrite}])
            .format(format)
            .build()?;
        Ok(image.get_memory_req()?.memory_types)
    }

    fn memory_type_for_buffer(
        &self,
        usage: base::BufferUsageFlags,
        optimal_caps: base::MemoryTypeCapsFlags,
        required_caps: base::MemoryTypeCapsFlags,
    ) -> Result<Option<base::MemoryType>> {
        let types = self.memory_types_for_buffer(usage)?;
        Ok(self.choose_memory_type(types, optimal_caps, required_caps))
    }

    fn memory_type_for_image(
        &self,
        format: base::ImageFormat,
        optimal_caps: base::MemoryTypeCapsFlags,
        required_caps: base::MemoryTypeCapsFlags,
    ) -> Result<Option<base::MemoryType>> {
        let types = self.memory_types_for_image(format)?;
        Ok(self.choose_memory_type(types, optimal_caps, required_caps))
    }
}
