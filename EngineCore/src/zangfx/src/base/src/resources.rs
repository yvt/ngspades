//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for (heap-allocated) resource objects, and other relevant types.
use std::ops;
use ngsenumflags::BitFlags;

use crate::sampler::Sampler;
use crate::formats::ImageFormat;
use crate::{Object, DeviceSize, Result};

define_handle! {
    /// Image handle.
    ///
    /// Images are first created using `ImageBuilder`. After an image is created
    /// it is in the **Prototype** state. Before it can be used as an attachment
    /// or a descriptor, it must first be transitioned to the **Allocated**
    /// state by allocating the physical space of the image via a method
    /// provided by `Heap`.
    ///
    /// Once an image is transitioned to the **Allocated** state, it will never
    /// go back to the original state. Destroying the heap where the image is
    /// located causes the image to transition to the **Invalid** state. The
    /// only valid operation to an image in the **Invalid** state is to destroy
    /// the image.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    Image
}

define_handle! {
    /// Buffer handle.
    ///
    /// Buffers are first created using `BufferBuilder`. After a buffer is created
    /// it is in the **Prototype** state. Before it can be used as an attachment
    /// or a descriptor, it must first be transitioned to the **Allocated**
    /// state by allocating the physical space of the buffer via a method
    /// provided by `Heap`.
    ///
    /// Once a buffer is transitioned to the **Allocated** state, it will never
    /// go back to the original state. Destroying the heap where the buffer is
    /// located causes the buffer to transition to the **Invalid** state. The
    /// only valid operation to a buffer in the **Invalid** state is to destroy
    /// the buffer.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    Buffer
}

define_handle! {
    /// Image view object.
    ///
    /// Shaders always access images via image views.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    ImageView
}

/// Trait for building images.
///
/// The image type is inferred from the property values. The following
/// combinations are permitted:
///
/// |  [Extents]  | [# of layers] | [Image type] |
/// | ----------- | ------------- | ------------ |
/// | `[x]`       | `None`        | 1D           |
/// | `[x]`       | `Some(i)`     | 1D array     |
/// | `[x, y]`    | `None`        | 2D           |
/// | `[x, y]`    | `Some(i)`     | 2D array     |
/// | `[x, y, z]` | `None`        | 3D           |
/// | [Cube]      | `None`        | Cube         |
/// | Cube        | `Some(i)`     | Cube array¹  |
///
/// ¹ Requires [`supports_cube_array`].
///
/// [Extents]: ImageBuilder::extents
/// [Cube]: ImageBuilder::extents_cube
/// [# of layers]: ImageBuilder::num_layers
/// [Image type]: ImageType
/// [`supports_cube_array`]: DeviceLimits::supports_cube_array
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
    /// Specifying one overwrites the specification of another.
    fn extents(&mut self, v: &[u32]) -> &mut ImageBuilder;

    /// Set the image extents to `v`. Used for cube images.
    ///
    /// Specifying either of `extents` and `extents_cube` is mandatory.
    /// Specifying one overwrites the specification of another.
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
    /// Must be less than or equal to
    /// `log2(extents_value.iter().max().unwrap()).ceil() + 1`. Defaults to `1`.
    ///
    /// Must be `1` for 1D textures.
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

impl From<ImageLayerRange> for ImageSubRange {
    fn from(x: ImageLayerRange) -> Self {
        Self {
            mip_levels: Some(x.mip_level..x.mip_level + 1),
            layers: Some(x.layers.clone()),
        }
    }
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

    /// Enables the creation of an `ImageView` with a different type (2D/3D/...).
    MutableType = 0b00100000,
    /// Enables the creation of an `ImageView` with a different image format.
    MutableFormat = 0b01000000,
    /// Enables the creation of an `ImageView` using a partial layer range of
    /// the original image.
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
    ///
    /// # Valid Usage
    ///
    ///  - The image must be in the Allocated state.
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
    ///
    /// [`usage`]: ImageBuilder::usage
    /// [`MutableFormat`]: ImageUsage::MutableFormat
    fn format(&mut self, v: ImageFormat) -> &mut ImageViewBuilder;

    /// Set the image view type.
    ///
    /// The original image's type is used by default. The original image's
    /// [`usage`] must include [`MutableType`] to specify a different type
    /// here.
    ///
    /// If `usage` includes `MutableType`, only the following combinations of
    /// the original image's `ImageType` and the image view's one are supported:
    ///
    /// | Original image type |          View image type          |
    /// | ------------------- | --------------------------------- |
    /// | 1D                  | 1D                                |
    /// | 2D or 2D array      | 2D or 2D array                    |
    /// | Cube or cube array  | 2D, 2D array, cube, or cube array |
    /// | 3D                  | 3D                                |
    ///
    /// [`usage`]: ImageBuilder::usage
    /// [`MutableType`]: ImageUsage::MutableType
    fn image_type(&mut self, v: ImageType) -> &mut ImageViewBuilder;

    /// Set the image layout.
    ///
    /// Defaults to `ImageLayout::ShaderRead`. Since image views are solely used
    /// for shader access, the only valid values here are `ShaderRead` and
    /// `General`.
    fn layout(&mut self, v: ImageLayout) -> &mut ImageViewBuilder;

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

/// A reference to a resource handle.
///
/// # Examples
///
///     # use zangfx_base::{Image, Buffer, ResourceRef};
///     fn test(image: Image, buffer: Buffer) {
///         let _ref1: ResourceRef = (&image).into();
///         let _ref2: ResourceRef = (&buffer).into();
///     }
///
#[derive(Debug, Clone, Copy)]
pub enum ResourceRef<'a> {
    Image(&'a Image),
    Buffer(&'a Buffer),
}

impl<'a> From<&'a Image> for ResourceRef<'a> {
    fn from(x: &'a Image) -> Self {
        ResourceRef::Image(x)
    }
}

impl<'a> From<&'a Buffer> for ResourceRef<'a> {
    fn from(x: &'a Buffer) -> Self {
        ResourceRef::Buffer(x)
    }
}

/// A reference to a homogeneous slice of handles that can be passed to a shader
/// function as an argument.
///
/// # Examples
///
///     # use zangfx_base::{ImageView, ArgSlice};
///     fn test(image1: ImageView, image2: ImageView) {
///         let _: ArgSlice = [&image1, &image2][..].into();
///     }
///
#[derive(Debug, Clone, Copy)]
pub enum ArgSlice<'a> {
    /// Image views.
    ImageView(&'a [&'a ImageView]),
    /// Buffers and their subranges.
    ///
    /// - For a uniform buffer, the starting offset of each range must be
    ///   aligned to `DeviceLimits::uniform_buffer_alignment` bytes.
    /// - For a storage buffer, the starting offset of each range must be
    ///   aligned to `DeviceLimits::storage_buffer_alignment` bytes.
    ///
    Buffer(&'a [(ops::Range<DeviceSize>, &'a Buffer)]),
    /// Samplers.
    Sampler(&'a [&'a Sampler]),
}

impl<'a> ArgSlice<'a> {
    pub fn len(&self) -> usize {
        match self {
            &ArgSlice::ImageView(x) => x.len(),
            &ArgSlice::Buffer(x) => x.len(),
            &ArgSlice::Sampler(x) => x.len(),
        }
    }
}

impl<'a> From<&'a [&'a ImageView]> for ArgSlice<'a> {
    fn from(x: &'a [&'a ImageView]) -> Self {
        ArgSlice::ImageView(x)
    }
}

impl<'a> From<&'a [(ops::Range<DeviceSize>, &'a Buffer)]> for ArgSlice<'a> {
    fn from(x: &'a [(ops::Range<DeviceSize>, &'a Buffer)]) -> Self {
        ArgSlice::Buffer(x)
    }
}

impl<'a> From<&'a [&'a Sampler]> for ArgSlice<'a> {
    fn from(x: &'a [&'a Sampler]) -> Self {
        ArgSlice::Sampler(x)
    }
}