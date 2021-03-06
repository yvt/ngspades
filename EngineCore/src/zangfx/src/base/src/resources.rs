//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for (heap-allocated) resource objects, and other relevant types.
use bitflags::bitflags;
use flags_macro::flags;
use std::ops;

use crate::command::CmdQueueRef;
use crate::formats::ImageFormat;
use crate::handles::CloneHandle;
use crate::sampler::SamplerRef;
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
    ImageRef: Image
}

/// Trait for image handles.
///
/// # State-tracking units
///
/// Images are stored in implementation-dependent layout in memory. Some
/// implementations support more than one layouts, each of which is optimized
/// for a particular set of operations. The backend and/or the device driver
/// track image layouts and perform layout conversion on-the-fly to optimize
/// the runtime performance. (ZanGFX reifies this concept via an enumerate type
/// [`ImageLayout`], which is only used in specific circumstances.)
///
/// A part of an image can have a different layout than others. However,
/// deciding at which granularity the tracking of imgae layouts should be done
/// is a difficult problem. Finer tracking takes more memory and leads to an
/// increased runtime overhead, while coarser tracking might lead to redundant
/// image layout transitions.
///
/// ZanGFX provides the applications with a mean to control the image layout
/// tracking granularity. The unit of tracking, a part of an image treated as
/// one as far as image layout tracking is concerned, is called a
/// **state-tracking unit**. By default, entire an image is considered a single
/// state-tracking unit. [`ImageUsageFlags`] includes flags to increase the
/// granularity.
///
/// In most operations, state-tracking units are transparent to the
/// applications. **The exceptions** are [`invalidate_image`],
/// [`queue_ownership_acquire`], and [`queue_ownership_release`] commands, which
/// override the internally tracked states for various purposes.
///
/// [`invalidate_image`]: crate::CmdBuffer::invalidate_image
/// [`queue_ownership_acquire`]: crate::CmdBuffer::queue_ownership_acquire
/// [`queue_ownership_release`]: crate::CmdBuffer::queue_ownership_release
///
/// # Additional fencing requirement
///
/// At any momenet, each image only can be in one layout. ZanGFX provides
/// automatic image layout tracking, which only happens at fencing boundaries.
/// For this reason, **you might need to perform extra fencing** in cases which
/// at first glance don't seem to need that.
///
/// The following pseudocode illustrates one of such cases:
///
/// ```text
/// render:
///     use(Image)                  (image = Shader)
///     update_fence(Fence1)
/// compute:
///     wait_fence(Fence1)
///     use(Image)                  (image = Shader)
/// copy:
///     wait_fence(Fence1)
///     copy_from_image_to_buffer(
///         Image, Buffer)          (image = Copy)
/// ```
///
/// In this example, the first two passes uses `Image` in the `Shader` layout
/// while the last one has to transition it into the `Copy` layout. Since there
/// isn't a fence defined between the second and third pass, the system might
/// try to perform layout transition while the image is still in use by the
/// second pass. The following modified pseudocode doesn't have this problem:
///
/// ```text
/// render:
///     use(Image)
///     update_fence(Fence1)
/// compute:
///     wait_fence(Fence1)
///     use(Image)
///     update_fence(Fence2)
/// copy:
///     wait_fence(Fence1)
///     wait_fence(Fence2)
///     copy_from_image_to_buffer(Image, Buffer)
/// ```
///
/// This additional fencing requirement does not apply to images marked with
/// [`ImageUsageFlags::Mutable`].
///
pub trait Image: CloneHandle<ImageRef> {
    /// Create a proxy object to use this image from a specified queue.
    ///
    /// The default implementation panics with a message indicating that the
    /// backend does not support inter-queue operation.
    ///
    /// # Valid Usage
    ///
    ///  - The image must not an image view.
    fn make_proxy(&self, queue: &CmdQueueRef) -> ImageRef {
        let _ = queue;
        panic!("Inter-queue operation is not supported by this backend.");
    }

    /// Create an `ImageViewBuilder` associated with this image.
    ///
    /// # Valid Usage
    ///
    ///  - The image must be in the Allocated state.
    fn build_image_view(&self) -> ImageViewBuilderRef;

    /// Retrieve the memory requirements for this image.
    ///
    /// # Valid Usage
    ///
    ///  - The image must not be an image view.
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
    BufferRef: Buffer
}

/// Trait for buffer handles.
pub unsafe trait Buffer: CloneHandle<BufferRef> {
    /// Create a proxy object to use this buffer from a specified queue.
    ///
    /// The default implementation panics with a message indicating that the
    /// backend does not support inter-queue operation.
    fn make_proxy(&self, queue: &CmdQueueRef) -> BufferRef {
        let _ = queue;
        panic!("Inter-queue operation is not supported by this backend.");
    }

    /// Get the address of the underlying storage of a buffer.
    ///
    /// The returned address must be valid throughout the lifetime of `self`.
    ///
    /// # Valid Usage
    ///
    ///  - The buffer must be in the **Allocated** state.
    ///  - The buffer must be bound to a heap whose memory type is host-visible.
    ///
    fn as_ptr(&self) -> *mut u8;

    /// Get the size of a buffer.
    fn len(&self) -> DeviceSize;

    /// Retrieve the memory requirements for this buffer.
    fn get_memory_req(&self) -> Result<MemoryReq>;
}

/// The builder object for images.
pub type ImageBuilderRef = Box<dyn ImageBuilder>;

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
/// [`supports_cube_array`]: crate::DeviceLimits::supports_cube_array
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
    fn queue(&mut self, queue: &CmdQueueRef) -> &mut dyn ImageBuilder;

    /// Set the image extents to `v`. Used for 1D/2D/3D images.
    ///
    /// `v.len()` matches the dimensionality of the image and must be one of
    /// 1, 2, and 3.
    ///
    /// Specifying either of `extents` and `extents_cube` is mandatory.
    /// Specifying one overwrites the specification of another.
    fn extents(&mut self, v: &[u32]) -> &mut dyn ImageBuilder;

    /// Set the image extents to `v`. Used for cube images.
    ///
    /// Specifying either of `extents` and `extents_cube` is mandatory.
    /// Specifying one overwrites the specification of another.
    fn extents_cube(&mut self, v: u32) -> &mut dyn ImageBuilder;

    /// Set the number of array layers.
    ///
    /// `None` indicates non-array image type. Defaults to `None`.
    ///
    /// `None` must be specified for 3D images (those for which a three-element
    /// slice was passed to `extents`).
    fn num_layers(&mut self, v: Option<u32>) -> &mut dyn ImageBuilder;

    /// Set the number of mipmap levels.
    ///
    /// Must be less than or equal to
    /// `log2(extents_value.iter().max().unwrap()).ceil() + 1`. Defaults to `1`.
    ///
    /// Must be `1` for 1D textures.
    fn num_mip_levels(&mut self, v: u32) -> &mut dyn ImageBuilder;

    /// Set the image format.
    ///
    /// This property is mandatory.
    fn format(&mut self, v: ImageFormat) -> &mut dyn ImageBuilder;

    /// Set the image usage.
    ///
    /// Defaults to `ImageUsageFlags::default()`
    /// (`flags![ImageUsageFlags::{CopyWrite | Sampled}]`).
    fn usage(&mut self, v: ImageUsageFlags) -> &mut dyn ImageBuilder;

    /// Build an `ImageRef`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<ImageRef>;
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

    /// Layout for using images as source of the copy commands defined by
    /// [`CopyCmdEncoder`].
    ///
    /// [`CopyCmdEncoder`]: crate::CopyCmdEncoder
    CopyRead,
    /// Layout for using images as destination of the copy commands defined by
    /// [`CopyCmdEncoder`].
    ///
    /// [`CopyCmdEncoder`]: crate::CopyCmdEncoder
    CopyWrite,
}

bitflags! {
    /// Specifies types of operations supported by an image.
    pub struct ImageUsageFlags: u16 {
        /// Enables uses of the image as the source of [copy commands].
        ///
        /// [copy commands]: crate::CopyCmdEncoder
        const COPY_READ = 0b00000001;
        /// Enables uses of the image as the destination of [copy commands].
        ///
        /// [copy commands]: crate::CopyCmdEncoder
        const COPY_WRITE = 0b00000010;
        /// Enables uses of the image as a [sampled image shader argument].
        ///
        /// [sampled image shader argument]: crate::ArgType::SampledImage
        const SAMPLED = 0b00000100;
        /// Enables uses of the image as a [storage image shader argument].
        ///
        /// Note: The [`use_heap`] command ignores images that include this usage
        /// flag.
        ///
        /// [storage image shader argument]: crate::ArgType::StorageImage
        /// [`use_heap`]: crate::CmdEncoder::use_heap
        const STORAGE = 0b00001000;
        /// Enables uses of the image as a render target.
        ///
        /// Note: The [`use_heap`] command ignores images that include this usage
        /// flag.
        ///
        /// [`use_heap`]: crate::CmdEncoder::use_heap
        const RENDER = 0b00010000;

        /// Enables the creation of an image view with a different type (2D/3D/...).
        const MUTABLE_TYPE = 0b00100000;
        /// Enables the creation of an image view with a different image format.
        const MUTABLE_FORMAT = 0b01000000;
        /// Enables the creation of an image view using a partial layer range of
        /// the original image.
        const PARTIAL_VIEW = 0b10000000;

        /// This flag serves as a hint that the backend should trade off the use of
        /// the generic image layout in memory for fewer image layout transitions.
        ///
        /// Using this flag removes [the additional fencing requirement].
        ///
        /// [the additional fencing requirement]: Image
        const MUTABLE = 0b100000000;

        /// Controls the size of [state-tracking units]. This flag instructs the
        /// backend to track the state of each mipmap level individually.
        ///
        /// [state-tracking units]: Image
        const TRACK_STATE_PER_MIPMAP_LEVEL = 0b1000000000;
        /// Controls the size of [state-tracking units]. This flag instructs the
        /// backend to track the state of each array layer individually.
        ///
        /// [state-tracking units]: Image
        const TRACK_STATE_PER_ARRAY_LAYER = 0b10000000000;
    }
}

impl Default for ImageUsageFlags {
    /// Get the default image usage flags used by [`ImageBuilder`](ImageBuilder).
    fn default() -> ImageUsageFlags {
        flags![ImageUsageFlags::{COPY_WRITE | SAMPLED}]
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum ImageAspect {
    Color,
    Depth,
    Stencil,
}

/// The builder object for buffers.
pub type BufferBuilderRef = Box<dyn BufferBuilder>;

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
    fn queue(&mut self, queue: &CmdQueueRef) -> &mut dyn BufferBuilder;

    /// Set the buffer size to `v` bytes.
    ///
    /// This property is mandatory.
    fn size(&mut self, v: DeviceSize) -> &mut dyn BufferBuilder;

    /// Set the buffer usage.
    ///
    /// Defaults to `BufferUsageFlags::default()`
    /// (`flags![BufferUsageFlags::{CopyWrite | Uniform}]`).
    fn usage(&mut self, v: BufferUsageFlags) -> &mut dyn BufferBuilder;

    /// Build a `BufferRef`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<BufferRef>;
}

bitflags! {
    pub struct BufferUsageFlags: u8 {
        const COPY_READ = 0b0000001;
        const COPY_WRITE = 0b0000010;
        const UNIFORM = 0b0000100;
        const STORAGE = 0b0001000;
        const INDEX = 0b0010000;
        const VERTEX = 0b0100000;
        const INDIRECT_DRAW = 0b1000000;
    }
}

impl Default for BufferUsageFlags {
    /// Get the default image usage flags used by `BufferBuilder`.
    fn default() -> BufferUsageFlags {
        flags![BufferUsageFlags::{COPY_WRITE | UNIFORM}]
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

/// The builder object for image views.
pub type ImageViewBuilderRef = Box<dyn ImageViewBuilder>;

/// Trait for building image views.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, image: ImageRef) {
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
    /// [`PartialView`]: ImageUsageFlags::PartialView
    fn subrange(&mut self, v: &ImageSubRange) -> &mut dyn ImageViewBuilder;

    /// Set the image view format.
    ///
    /// The original image's format is used by default. The original image's
    /// [`usage`] must include [`MutableFormat`] to specify a different format
    /// here.
    ///
    /// [`usage`]: ImageBuilder::usage
    /// [`MutableFormat`]: ImageUsageFlags::MutableFormat
    fn format(&mut self, v: ImageFormat) -> &mut dyn ImageViewBuilder;

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
    /// [`MutableType`]: ImageUsageFlags::MutableType
    fn image_type(&mut self, v: ImageType) -> &mut dyn ImageViewBuilder;

    /// Build an image view.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<ImageRef>;
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
/// The name is actually a misnormer; `ResourceRefRef` would be more accurate,
/// albeit being weird.
///
/// # Examples
///
///     # use zangfx_base::{ImageRef, BufferRef, ResourceRef};
///     fn test(image: ImageRef, buffer: BufferRef) {
///         let _ref1: ResourceRef = (&image).into();
///         let _ref2: ResourceRef = (&buffer).into();
///     }
///
#[derive(Debug, Clone, Copy)]
pub enum ResourceRef<'a> {
    Image(&'a ImageRef),
    Buffer(&'a BufferRef),
}

impl<'a> ResourceRef<'a> {
    /// Return `Some(x)` for `ResourceRef::Image(x)`; `None` otherwise.
    pub fn image(&self) -> Option<&'a ImageRef> {
        match self {
            ResourceRef::Image(x) => Some(x),
            ResourceRef::Buffer(_) => None,
        }
    }

    /// Return `Some(x)` for `ResourceRef::Buffer(x)`; `None` otherwise.
    pub fn buffer(&self) -> Option<&'a BufferRef> {
        match self {
            ResourceRef::Buffer(x) => Some(x),
            ResourceRef::Image(_) => None,
        }
    }

    /// Retrieve the memory requirements for this resource.
    pub fn get_memory_req(&self) -> Result<MemoryReq> {
        match self {
            ResourceRef::Buffer(x) => x.get_memory_req(),
            ResourceRef::Image(x) => x.get_memory_req(),
        }
    }
}

impl<'a> From<&'a ImageRef> for ResourceRef<'a> {
    fn from(x: &'a ImageRef) -> Self {
        ResourceRef::Image(x)
    }
}

impl<'a> From<&'a BufferRef> for ResourceRef<'a> {
    fn from(x: &'a BufferRef) -> Self {
        ResourceRef::Buffer(x)
    }
}

/// A set of references to resource handles.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(image: ImageRef, buffer: BufferRef) {
///     // Empty
///     let _: ResourceSet = ().into();
///
///     // Single resource
///     let _: ResourceSet = (&image).into();
///     let _: ResourceSet = (&buffer).into();
///
///     // Homogeneous list
///     let _: ResourceSet = (&[&image, &image][..]).into();
///     let _: ResourceSet = (&[&buffer, &buffer][..]).into();
///
///     // Heterogeneous list
///     let _: ResourceSet = (&resources![&image, &buffer][..]).into();
///     # }
///
#[derive(Debug, Clone, Copy)]
pub enum ResourceSet<'a> {
    Empty,
    Image([&'a ImageRef; 1]),
    Buffer([&'a BufferRef; 1]),
    Images(&'a [&'a ImageRef]),
    Buffers(&'a [&'a BufferRef]),
    Resources(&'a [ResourceRef<'a>]),
}

/// Constructs an `[ResourceRef; _]`, converting all elements to `ResourceRef`.
#[macro_export]
macro_rules! resources {
    ( $($x:expr),* $(,)* ) => ( [$($crate::ResourceRef::from($x)),*] )
}

impl<'a> ResourceSet<'a> {
    /// Get an iterator that visits all resources in the `ResourceSet`.
    pub fn iter<'b>(&'b self) -> impl Iterator<Item = ResourceRef<'_>> + 'b {
        let mut images = &[][..];
        let mut buffers = &[][..];
        let mut hetero = &[][..];
        match self {
            ResourceSet::Empty => {}
            ResourceSet::Image(a) => images = &a[..],
            ResourceSet::Buffer(a) => buffers = &a[..],
            ResourceSet::Images(a) => images = a,
            ResourceSet::Buffers(a) => buffers = a,
            ResourceSet::Resources(a) => hetero = &a,
        }
        hetero
            .iter()
            .cloned()
            .chain(images.iter().cloned().map(|e| ResourceRef::from(e)))
            .chain(buffers.iter().cloned().map(|e| ResourceRef::from(e)))
    }

    /// Get an iterator that visits all images in the `ResourceSet`.
    pub fn images<'b>(&'b self) -> impl Iterator<Item = &'a ImageRef> + 'b {
        let mut images = &[][..];
        let mut hetero = &[][..];
        match self {
            ResourceSet::Image(a) => images = &a[..],
            ResourceSet::Images(a) => images = a,
            ResourceSet::Resources(a) => hetero = &a,
            _ => {}
        }
        hetero
            .iter()
            .filter_map(ResourceRef::image)
            .chain(images.iter().cloned())
    }

    /// Get an iterator that visits all buffers in the `ResourceSet`.
    pub fn buffers<'b>(&'b self) -> impl Iterator<Item = &'a BufferRef> + 'b {
        let mut buffers = &[][..];
        let mut hetero = &[][..];
        match self {
            ResourceSet::Buffer(a) => buffers = &a[..],
            ResourceSet::Buffers(a) => buffers = a,
            ResourceSet::Resources(a) => hetero = &a,
            _ => {}
        }
        hetero
            .iter()
            .filter_map(ResourceRef::buffer)
            .chain(buffers.iter().cloned())
    }
}

impl<'a> From<()> for ResourceSet<'a> {
    fn from(_: ()) -> Self {
        ResourceSet::Empty
    }
}

impl<'a> From<&'a ImageRef> for ResourceSet<'a> {
    fn from(x: &'a ImageRef) -> Self {
        ResourceSet::Image([x])
    }
}

impl<'a> From<&'a BufferRef> for ResourceSet<'a> {
    fn from(x: &'a BufferRef) -> Self {
        ResourceSet::Buffer([x])
    }
}

impl<'a> From<&'a [&'a ImageRef]> for ResourceSet<'a> {
    fn from(x: &'a [&'a ImageRef]) -> Self {
        ResourceSet::Images(x)
    }
}

impl<'a> From<&'a [&'a BufferRef]> for ResourceSet<'a> {
    fn from(x: &'a [&'a BufferRef]) -> Self {
        ResourceSet::Buffers(x)
    }
}

impl<'a> From<&'a [ResourceRef<'a>]> for ResourceSet<'a> {
    fn from(x: &'a [ResourceRef<'a>]) -> Self {
        ResourceSet::Resources(x)
    }
}

/// A reference to a homogeneous slice of handles that can be passed to a shader
/// function as an argument.
///
/// # Examples
///
///     # use zangfx_base::{ImageRef, ArgSlice};
///     fn test(image1: ImageRef, image2: ImageRef) {
///         let _: ArgSlice = [&image1, &image2][..].into();
///     }
///
#[derive(Debug, Clone, Copy)]
pub enum ArgSlice<'a> {
    /// Images.
    Image(&'a [&'a ImageRef]),
    /// Buffers and their subranges.
    ///
    /// - For a uniform buffer, the starting offset of each range must be
    ///   aligned to `DeviceLimits::uniform_buffer_alignment` bytes.
    /// - For a storage buffer, the starting offset of each range must be
    ///   aligned to `DeviceLimits::storage_buffer_alignment` bytes.
    ///
    Buffer(&'a [(ops::Range<DeviceSize>, &'a BufferRef)]),
    /// Samplers.
    Sampler(&'a [&'a SamplerRef]),
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

impl<'a> From<&'a [&'a ImageRef]> for ArgSlice<'a> {
    fn from(x: &'a [&'a ImageRef]) -> Self {
        ArgSlice::Image(x)
    }
}

impl<'a> From<&'a [(ops::Range<DeviceSize>, &'a BufferRef)]> for ArgSlice<'a> {
    fn from(x: &'a [(ops::Range<DeviceSize>, &'a BufferRef)]) -> Self {
        ArgSlice::Buffer(x)
    }
}

impl<'a> From<&'a [&'a SamplerRef]> for ArgSlice<'a> {
    fn from(x: &'a [&'a SamplerRef]) -> Self {
        ArgSlice::Sampler(x)
    }
}
