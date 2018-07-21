//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for (heap-allocated) resource objects, and other relevant types.
use ngsenumflags::BitFlags;
use std::ops;

use crate::formats::ImageFormat;
use crate::sampler::Sampler;
use crate::handles::HandleImpl;
use crate::command::CmdQueue;
use crate::{DeviceSize, Object, Result};

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
    Image: ImageTrait
}

/// Trait for image handles.
pub trait ImageTrait: HandleImpl<Image> {
    /// Create a proxy object to use this image from a specified queue.
    ///
    /// # Valid Usage
    ///
    ///  - The image must not an image view.
    fn make_proxy(&mut self, queue: &CmdQueue) -> Image;

    /// Create an `ImageViewBuilder` associated with this image.
    ///
    /// # Valid Usage
    ///
    ///  - The image must be in the Allocated state.
    fn build_image_view(&self) -> Box<ImageViewBuilder>;

    /// Retrieve the memory requirements for this image.
    fn get_memory_req(&self) -> Result<MemoryReq>;
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

/// Trait for buffer handles.
pub trait BufferTrait: HandleImpl<Buffer> {
    /// Create a proxy object to use this buffer from a specified queue.
    fn make_proxy(&mut self, queue: &CmdQueue) -> Buffer;

    /// Get the address of the underlying storage of a buffer.
    ///
    /// # Valid Usage
    ///
    ///  - The buffer must be in the **Allocated** state.
    ///  - The buffer must be bound to a heap whose memory type is host-visible.
    ///
    fn as_ptr(&self) -> *mut u8;

    /// Retrieve the memory requirements for this buffer.
    fn get_memory_req(&self) -> Result<MemoryReq>;
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
    /// Specify the queue associated with the created image.
    ///
    /// Defaults to the backend-specific value.
    fn queue(&mut self, queue: &CmdQueue) -> &mut ImageBuilder;

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

/// Specifies an image layout.
///
/// Images are stored in implementation-dependent layouts in memory. Each image
/// layout supports a particular set of operations. Although in most cases
/// layout transitions are automatic, there are some cases where explicitly
/// specifying a layout can lead to a more optimal operation of a device.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageLayout {
    // TODO: Read-only render targets

    /// Layout for render targets.
    Render,

    /// Layout for accesses from shaders.
    Shader,

    /// Layout for using images as source of the copy commanddefined by
    /// [`CopyCmdEncoder`].
    ///
    /// [`CopyCmdEncoder`]: CopyCmdEncoder
    CopyRead,
    /// Layout for using images as destination of the copy command defined by
    /// [`CopyCmdEncoder`].
    ///
    /// [`CopyCmdEncoder`]: CopyCmdEncoder
    CopyWrite,
}

/// Specifies a type of operations supported by an image.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageUsage {
    /// Enables uses of the image as the source of [copy commands].
    ///
    /// [copy commands]: crate::CopyCmdEncoder
    CopyRead = 0b00000001,
    /// Enables uses of the image as the destination of [copy commands].
    ///
    /// [copy commands]: crate::CopyCmdEncoder
    CopyWrite = 0b00000010,
    /// Enables uses of the image as a [sampled image shader argument].
    ///
    /// [sampled image shader argument]: crate::ArgType::SampledImage
    Sampled = 0b00000100,
    /// Enables uses of the image as a [storage image shader argument].
    ///
    /// Note: The [`use_heap`] command ignores images that include this usage
    /// flag.
    ///
    /// [storage image shader argument]: crate::ArgType::StorageImage
    /// [`use_heap`]: crate::CmdEncoder::use_heap
    Storage = 0b00001000,
    /// Enables uses of the image as a render target.
    ///
    /// Note: The [`use_heap`] command ignores images that include this usage
    /// flag.
    ///
    /// [`use_heap`]: crate::CmdEncoder::use_heap
    Render = 0b00010000,

    /// Enables the creation of an image view with a different type (2D/3D/...).
    MutableType = 0b00100000,
    /// Enables the creation of an image view with a different image format.
    MutableFormat = 0b01000000,
    /// Enables the creation of an image view using a partial layer range of
    /// the original image.
    PartialView = 0b10000000,

    /// This flag serves as a hint that the backend should trade off the use of
    /// the generic image layout in memory for fewer image layout transitions.
    Mutable = 0b100000000,

    /// This flag serves as a hint that the backend should track the state of
    /// each mipmap level individually.
    TrackStatePerMipmapLevel = 0b1000000000,
    /// This flag serves as a hint that the backend should track the state of
    /// each array layer individually.
    TrackStatePerArrayLayer = 0b10000000000,
}

/// Specifies types of operations supported by an image.
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
    /// Specify the queue associated with the created buffer.
    ///
    /// Defaults to the backend-specific value.
    fn queue(&mut self, queue: &CmdQueue) -> &mut BufferBuilder;

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
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, image: Image) {
///     let image_view = image.build_image_view()
///         .subrange(&ImageSubRange {
///             mip_levels: Some(0..1),
///             layers: Some(0..1),
///         })
///         .build()
///         .expect("Failed to create an image view.");
///     # }
///
pub trait ImageViewBuilder: Object {
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

    /// Build an image view.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<Image>;
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
///     # use zangfx_base::{Image, ArgSlice};
///     fn test(image1: Image, image2: Image) {
///         let _: ArgSlice = [&image1, &image2][..].into();
///     }
///
#[derive(Debug, Clone, Copy)]
pub enum ArgSlice<'a> {
    /// Images.
    Image(&'a [&'a Image]),
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
            &ArgSlice::Image(x) => x.len(),
            &ArgSlice::Buffer(x) => x.len(),
            &ArgSlice::Sampler(x) => x.len(),
        }
    }
}

impl<'a> From<&'a [&'a Image]> for ArgSlice<'a> {
    fn from(x: &'a [&'a Image]) -> Self {
        ArgSlice::Image(x)
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
