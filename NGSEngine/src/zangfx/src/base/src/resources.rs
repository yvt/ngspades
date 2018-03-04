//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for (heap-allocated) resource objects, and other relevant types.
use Object;
use std::ops;
use ngsenumflags::BitFlags;

use common::Result;
use handles::{Buffer, Image, ImageView};
use formats::ImageFormat;
use DeviceSize;

/// Trait for building images.
///
/// # Valid Usage
///
///  - No instance of `ImageBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::formats::ImageFormat;
///     # use zangfx_base::resources::ImageBuilder;
///     # fn test(device: &Device) {
///     let image = device.build_image()
///         .extents(&[1024, 768])
///         .format(ImageFormat::SrgbBgra8)
///         .build()
///         .expect("Failed to create an image.");
///     # }
///
pub trait ImageBuilder: Object {
    /// Set the image extents to `v`. Used for 1D/2D/3D images.
    ///
    /// `v.len()` matches the dimensionality of the image and must be one of
    /// 1, 2, and 3.
    ///
    /// Specifying either of `extents` and `extents_cube` is mandatory.
    fn extents(&mut self, v: &[u32]) -> &mut ImageBuilder;

    /// Set the image extents to `v`. Used for cube images.
    ///
    /// Specifying either of `extents` and `extents_cube` is mandatory.
    fn extents_cube(&mut self, v: u32) -> &mut ImageBuilder;

    /// Set the number of array layers.
    ///
    /// `None` indicates non-array image type. Defaults to `None`.
    ///
    /// `None` must be specified for 3D images (those for which a three-element
    /// slice was passed to `extents`).
    fn num_layers(&mut self, v: Option<u32>) -> &mut ImageBuilder;

    /// Set the number of mipmap levels.
    ///
    /// Must be less than or equal to `log2(extents_value.iter().max())`.
    /// Defaults to `1`.
    fn num_mip_levels(&mut self, v: u32) -> &mut ImageBuilder;

    /// Set the image format.
    ///
    /// This property is mandatory.
    fn format(&mut self, v: ImageFormat) -> &mut ImageBuilder;

    /// Set the image usage.
    ///
    /// Defaults to `ImageUsage::default_flags()`
    /// (`ImageUsage::CopyWrite | ImageUsage::Sampled`).
    fn usage(&mut self, v: ImageUsageFlags) -> &mut ImageBuilder;

    /// Build an `Image`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<Image>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ImageSubRange {
    /// The mipmap level(s). Use [`None`](None) to specify all levels.
    pub mip_levels: Option<ops::Range<u32>>,

    /// The array layer(s) accessible to the view. Use [`None`](None) to specify
    /// all layers.
    pub layers: Option<ops::Range<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageLayerRange {
    /// The mipmap level to use.
    pub mip_level: u32,

    /// The array layer(s) to use.
    pub layers: ops::Range<u32>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageLayout {
    Undefined,
    General,
    RenderRead,
    RenderWrite,
    ShaderRead,
    CopyRead,
    CopyWrite,
    Present,
}

#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageUsage {
    CopyRead = 0b00000001,
    CopyWrite = 0b00000010,
    Sampled = 0b00000100,
    Storage = 0b00001000,
    Render = 0b00010000,

    /// Enables the creation of `ImageView` with a different type (2D/3D/...).
    MutableType = 0b00100000,
    /// Enables the creation of `ImageView` with a different image format.
    MutableFormat = 0b01000000,
    /// Enables the creation of `ImageView` using a partial layer range of the
    /// original image.
    PartialView = 0b10000000,
}

pub type ImageUsageFlags = BitFlags<ImageUsage>;

impl ImageUsage {
    /// Get the default image usage flags used by [`ImageBuilder`](ImageBuilder).
    pub fn default_flags() -> ImageUsageFlags {
        ImageUsage::CopyWrite | ImageUsage::Sampled
    }
}

#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageAspect {
    Color = 0b001,
    Depth = 0b010,
    Stencil = 0b100,
}

/// Trait for building buffers.
///
/// # Valid Usage
///
///  - No instance of `BufferBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::resources::BufferBuilder;
///     # fn test(device: &Device) {
///     let buffer = device.build_buffer()
///         .size(1024 * 1024)
///         .build()
///         .expect("Failed to create a buffer.");
///     # }
///
pub trait BufferBuilder: Object {
    /// Set the buffer size to `v` bytes.
    ///
    /// This property is mandatory.
    fn size(&mut self, v: DeviceSize) -> &mut BufferBuilder;

    /// Set the buffer usage.
    ///
    /// Defaults to `BufferUsage::default_flags()`
    /// (`BufferUsage::CopyWrite | BufferUsage::Uniform`).
    fn usage(&mut self, v: BufferUsageFlags) -> &mut BufferBuilder;

    /// Build a `Buffer`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<Buffer>;
}

#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum BufferUsage {
    CopyRead = 0b0000001,
    CopyWrite = 0b0000010,
    Uniform = 0b0000100,
    Storage = 0b0001000,
    Index = 0b0010000,
    Vertex = 0b0100000,
    IndirectDraw = 0b1000000,
}

pub type BufferUsageFlags = BitFlags<BufferUsage>;

impl BufferUsage {
    /// Get the default image usage flags used by `ImageBuilder`.
    pub fn default_flags() -> BufferUsageFlags {
        BufferUsage::CopyWrite | BufferUsage::Uniform
    }
}

/// Memory requirements of a resource.
#[derive(Debug, Clone, Copy)]
pub struct MemoryReq {
    /// The number of bytes required for the memory allocation for the resource.
    pub size: DeviceSize,

    /// The required alignment of the resource (measured in bytes).
    pub align: DeviceSize,

    /// The set of memory types supported by the resource. Each bit corresponds
    /// to a single memory type.
    ///
    /// # Examples
    ///
    ///     # extern crate zangfx_base;
    ///     # extern crate zangfx_common;
    ///     # fn main() {
    ///     # use zangfx_base::resources::MemoryReq;
    ///     use zangfx_base::MemoryType;
    ///     fn supports_memory_type(req: &MemoryReq, ty: MemoryType) -> bool {
    ///         use zangfx_common::BinaryInteger;
    ///         req.memory_types.get_bit(ty)
    ///     }
    ///     # }
    ///
    pub memory_types: u32,
}

/// Trait for building image views.
///
/// # Valid Usage
///
///  - No instance of `ImageViewBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, image: Image) {
///     let image_view = device.build_image_view()
///         .image(&image)
///         .build()
///         .expect("Failed to create an image view.");
///     # }
///
pub trait ImageViewBuilder: Object {
    /// Set the image.
    ///
    /// This property is mandatory.
    fn image(&mut self, v: &Image) -> &mut ImageViewBuilder;

    /// Set the subresource range to `v`.
    ///
    /// Defaults to `Default::default()` (full range). The original image's
    /// [`usage`] must include [`PartialView`] to specify a partial range here.
    ///
    /// [`usage`]: ImageBuilder::usage
    /// [`PartialView`]: ImageUsage::PartialView
    fn subrange(&mut self, v: &ImageSubRange) -> &mut ImageViewBuilder;

    /// Set the image view format.
    ///
    /// The original image's format is used by default. The original image's
    /// [`usage`] must include [`MutableFormat`] to specify a different format
    /// here.
    fn format(&mut self, v: ImageFormat) -> &mut ImageViewBuilder;

    /// Set the image view type.
    ///
    /// The original image's type is used by default. The original image's
    /// [`usage`] must include [`MutableType`] to specify a different type
    /// here.
    fn image_type(&mut self, v: ImageType) -> &mut ImageViewBuilder;

    /// Build an `ImageView`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<ImageView>;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageType {
    OneD,
    TwoD,
    TwoDArray,
    ThreeD,
    Cube,
    CubeArray,
}
