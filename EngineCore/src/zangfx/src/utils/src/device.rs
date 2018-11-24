//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use flags_macro::flags;
use std::result::Result as StdResult;

use zangfx_base::{self as base, Error, Result};
use zangfx_common::BinaryInteger;

/// An extension trait for `Device`.
pub trait DeviceUtils: base::Device {
    /// Find the optimal memory type specific to the device based on the
    /// requirements.
    ///
    /// - `valid_memory_types` represents a set of memory types valid for your
    ///   intended usage.
    /// - `optimal_caps` represents the capabilities of the memory type
    ///   preferred by the application.
    /// - `required_caps` represents the capabilities the memory type must have.
    ///
    /// # Examples
    ///
    ///     use flags_macro::flags;
    ///     use zangfx_base::*;
    ///     use zangfx_utils::DeviceUtils;
    ///     # fn test(
    ///     #     device: &Device,
    ///     # ) -> Result<()> {
    ///     let buffer = device.build_buffer()
    ///         .size(64 as u64)
    ///         .usage(flags![BufferUsageFlags::{Vertex}])
    ///         .build()?;
    ///
    ///     let memory_type = device
    ///         .choose_memory_type(
    ///             buffer.get_memory_req()?.memory_types,
    ///             flags![MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
    ///             flags![MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
    ///         )
    ///         .expect("suitable memory type was not found");
    ///
    ///     assert!(
    ///         device.global_heap(memory_type).bind((&buffer).into())?,
    ///         "allocation failed",
    ///     );
    ///     # Ok(())
    ///     # }
    fn choose_memory_type(
        &self,
        valid_memory_types: impl TryValidMemoryTypes<Error = !>,
        optimal_caps: base::MemoryTypeCapsFlags,
        required_caps: base::MemoryTypeCapsFlags,
    ) -> Option<base::MemoryType> {
        self.try_choose_memory_type(valid_memory_types, optimal_caps, required_caps)
            .unwrap()
    }

    /// Find the optimal memory type specific to the device based on the
    /// requirements.
    ///
    /// - `valid_memory_types` represents a set of memory types valid for your
    ///   intended usage. It can be a reference to a resource handle.
    /// - `optimal_caps` represents the capabilities of the memory type
    ///   preferred by the application.
    /// - `required_caps` represents the capabilities the memory type must have.
    ///
    /// # Guarantees
    ///
    /// Backend implementation ensure the following as per Vulkan 1.0
    /// "11.6. Resource Memory Association".
    ///
    /// If [`zangfx_base::BufferUsageFlags`] is provided as `valid_memory_types`,
    /// unless any error occurs during the process, this method is guaranteed to
    /// return some memory type provided that `required_caps` is a subset of at
    /// least one of the following: `DeviceLocal` and
    /// `HostVisible | HostCoherent`.
    ///
    /// If [`zangfx_base::ImageFormat`] is provided as `valid_memory_types`,
    /// unless any error occurs during the process, this method is guaranteed to
    /// return some memory type provided that `required_caps` is a subset of at
    /// least one of the following: `DeviceLocal`.
    ///
    /// Note: Images are never host-visible in ZanGFX.
    ///
    /// # Examples
    ///
    ///     use flags_macro::flags;
    ///     use zangfx_base::*;
    ///     use zangfx_utils::DeviceUtils;
    ///     # fn test(
    ///     #     device: &Device,
    ///     # ) -> Result<()> {
    ///     // Create a buffer. At this point, this buffer is not bound to any heap yet.
    ///     let buffer = device.build_buffer()
    ///         .size(64 as u64)
    ///         .usage(flags![BufferUsageFlags::{Vertex}])
    ///         .build()?;
    ///
    ///     // Figure out the best memory type for this buffer
    ///     let memory_type = device
    ///         .try_choose_memory_type(
    ///             &buffer,
    ///             flags![MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
    ///             flags![MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
    ///         )?
    ///         .expect("suitable memory type was not found");
    ///
    ///     // Bind the buffer to the heap of that memory type
    ///     assert!(
    ///         device.global_heap(memory_type).bind((&buffer).into())?,
    ///         "allocation failed",
    ///     );
    ///     # Ok(())
    ///     # }
    ///
    /// Or, alternatively:
    ///
    ///     # use flags_macro::flags;
    ///     # use zangfx_base::*;
    ///     # use zangfx_utils::DeviceUtils;
    ///     # fn test(
    ///     #     device: &Device,
    ///     # ) -> Result<()> {
    ///     let memory_type = device
    ///         .try_choose_memory_type(
    ///             flags![BufferUsageFlags::{Vertex}],
    ///             flags![MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
    ///             flags![MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
    ///         )?
    ///         .expect("suitable memory type was not found");
    ///     # Ok(())
    ///     # }
    ///
    /// You can use `ImageFormat` too:
    ///
    ///     # use flags_macro::flags;
    ///     # use zangfx_base::*;
    ///     # use zangfx_utils::DeviceUtils;
    ///     # fn test(
    ///     #     device: &Device,
    ///     # ) -> Result<()> {
    ///     let memory_type = device
    ///         .try_choose_memory_type(
    ///             ImageFormat::SrgbBgra8,
    ///             flags![MemoryTypeCapsFlags::{DeviceLocal}],
    ///             flags![MemoryTypeCapsFlags::{}],
    ///         )?
    ///         .expect("suitable memory type was not found");
    ///     # Ok(())
    ///     # }
    ///
    fn try_choose_memory_type<T: TryValidMemoryTypes>(
        &self,
        valid_memory_types: T,
        optimal_caps: base::MemoryTypeCapsFlags,
        required_caps: base::MemoryTypeCapsFlags,
    ) -> StdResult<Option<base::MemoryType>, T::Error> {
        let valid_memory_types = valid_memory_types.try_valid_memory_types(self)?;

        // Based on the algorithm shown in Vulkan specification 1.0
        // "10.2. Device Memory".
        let memory_types = self.caps().memory_types();

        for i in valid_memory_types.one_digits() {
            if memory_types[i as usize].caps.contains(optimal_caps) {
                return Ok(Some(i));
            }
        }

        for i in valid_memory_types.one_digits() {
            if memory_types[i as usize].caps.contains(required_caps) {
                return Ok(Some(i));
            }
        }

        Ok(None)
    }

    /// A shortcut method of `choose_memory_type` for shared, coherent
    /// accesses by the host and device (`HostVisible | HostCoherent`).
    fn choose_memory_type_shared(
        &self,
        valid_memory_types: impl TryValidMemoryTypes<Error = !>,
    ) -> Option<base::MemoryType> {
        self.try_choose_memory_type_shared(valid_memory_types)
            .unwrap()
    }

    /// A shortcut method of `try_choose_memory_type` for shared, coherent
    /// accesses by the host and device (`HostVisible | HostCoherent`).
    ///
    /// Note: Images are never host-visible in ZanGFX.
    ///
    /// # Examples
    ///
    ///     # use flags_macro::flags;
    ///     # use zangfx_base::*;
    ///     # use zangfx_utils::DeviceUtils;
    ///     # fn test(
    ///     #     device: &Device,
    ///     # ) -> Result<()> {
    ///     let memory_type = device
    ///         .try_choose_memory_type_shared(ImageFormat::SrgbBgra8)?
    ///         .expect("suitable memory type was not found - this should never happen!");
    ///     # Ok(())
    ///     # }
    fn try_choose_memory_type_shared<T: TryValidMemoryTypes>(
        &self,
        valid_memory_types: T,
    ) -> StdResult<Option<base::MemoryType>, T::Error> {
        self.try_choose_memory_type(
            valid_memory_types,
            flags![base::MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
            flags![base::MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
        )
    }

    /// A shortcut method of `choose_memory_type` for private accesses by the
    /// device (`DeviceLocal`).
    fn choose_memory_type_private(
        &self,
        valid_memory_types: impl TryValidMemoryTypes<Error = !>,
    ) -> Option<base::MemoryType> {
        self.try_choose_memory_type_private(valid_memory_types)
            .unwrap()
    }

    /// A shortcut method of `try_choose_memory_type` for private accesses by
    /// the device (`DeviceLocal`).
    ///
    /// Note: Images are never host-visible in ZanGFX.
    ///
    /// # Examples
    ///
    ///     # use flags_macro::flags;
    ///     # use zangfx_base::*;
    ///     # use zangfx_utils::DeviceUtils;
    ///     # fn test(
    ///     #     device: &Device,
    ///     # ) -> Result<()> {
    ///     let memory_type = device
    ///         .try_choose_memory_type_shared(flags![BufferUsageFlags::{Vertex}])?
    ///         .expect("suitable memory type was not found - this should never happen!");
    ///     # Ok(())
    ///     # }
    fn try_choose_memory_type_private<T: TryValidMemoryTypes>(
        &self,
        valid_memory_types: T,
    ) -> StdResult<Option<base::MemoryType>, T::Error> {
        self.try_choose_memory_type(
            valid_memory_types,
            flags![base::MemoryTypeCapsFlags::{DeviceLocal}],
            flags![base::MemoryTypeCapsFlags::{}],
        )
    }

    /// Get the supported memory types for the given buffer usage.
    fn memory_types_for_buffer(&self, usage: base::BufferUsageFlags) -> Result<u32> {
        let buffer = self.build_buffer().size(1).usage(usage).build()?;
        Ok(buffer.get_memory_req()?.memory_types)
    }

    /// Get the supported memory types for the given image format.
    ///
    /// The returned values are guaranteed to be identical for all color
    /// formats. This, however, does not apply to depth/stencil formats.
    /// The backend implementation should ensure that as per Vulkan 1.0
    /// "11.6. Resource Memory Association".
    fn memory_types_for_image(&self, format: base::ImageFormat) -> Result<u32> {
        let image = self
            .build_image()
            .extents(&[1, 1])
            .usage(flags![base::ImageUsageFlags::{CopyRead | CopyWrite}])
            .format(format)
            .build()?;
        Ok(image.get_memory_req()?.memory_types)
    }
}

impl<T: base::Device + ?Sized> DeviceUtils for T {}

/// An object from which a set of supported memory types can be determined,
/// with fallibility.
///
/// It can be one of the following:
///
///  - [`zangfx_base::BufferUsageFlags`]
///  - [`zangfx_base::ImageFormat`]¹
///  - A reference to a resource (e.g., `&`[`zangfx_base::ImageRef`]).
///  - A bit-field representing the memory type set itself (i.e., [`u32`]).
///
/// ¹ The returned values are guaranteed to be identical for all color
/// formats. This, however, does not apply to depth/stencil formats.
/// The backend implementation should ensure that as per Vulkan 1.0
/// "11.6. Resource Memory Association".
///
/// This trait is used by the method [`DeviceUtils::try_choose_memory_type`] and
/// its family.
pub trait TryValidMemoryTypes {
    type Error;

    /// Get a set of valid memory types.
    fn try_valid_memory_types(
        &self,
        device: &(impl base::Device + ?Sized),
    ) -> StdResult<u32, Self::Error>;
}

impl TryValidMemoryTypes for u32 {
    type Error = !;
    fn try_valid_memory_types(
        &self,
        _device: &(impl base::Device + ?Sized),
    ) -> StdResult<u32, Self::Error> {
        Ok(*self)
    }
}

impl<'a> TryValidMemoryTypes for &'a base::ImageRef {
    type Error = Error;
    fn try_valid_memory_types(
        &self,
        _device: &(impl base::Device + ?Sized),
    ) -> StdResult<u32, Self::Error> {
        Ok(self.get_memory_req()?.memory_types)
    }
}

impl<'a> TryValidMemoryTypes for &'a base::BufferRef {
    type Error = Error;
    fn try_valid_memory_types(
        &self,
        _device: &(impl base::Device + ?Sized),
    ) -> StdResult<u32, Self::Error> {
        Ok(self.get_memory_req()?.memory_types)
    }
}

impl<'a> TryValidMemoryTypes for base::ResourceRef<'a> {
    type Error = Error;
    fn try_valid_memory_types(
        &self,
        _device: &(impl base::Device + ?Sized),
    ) -> StdResult<u32, Self::Error> {
        Ok(self.get_memory_req()?.memory_types)
    }
}

impl TryValidMemoryTypes for base::BufferUsageFlags {
    type Error = Error;
    fn try_valid_memory_types(
        &self,
        device: &(impl base::Device + ?Sized),
    ) -> StdResult<u32, Self::Error> {
        device.memory_types_for_buffer(*self)
    }
}

impl TryValidMemoryTypes for base::ImageFormat {
    type Error = Error;
    fn try_valid_memory_types(
        &self,
        device: &(impl base::Device + ?Sized),
    ) -> StdResult<u32, Self::Error> {
        device.memory_types_for_image(*self)
    }
}
