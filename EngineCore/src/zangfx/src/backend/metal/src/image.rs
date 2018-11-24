//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Image` for Metal.
use std::cell::UnsafeCell;
use std::ops;
use std::sync::Arc;

use cocoa::foundation::NSRange;
use flags_macro::flags;
use zangfx_base::{self as base, Result};
use zangfx_base::{zangfx_impl_handle, zangfx_impl_object};
use zangfx_metal_rs as metal;

use crate::formats::{translate_image_format, translate_metal_pixel_format};
use crate::utils::{nil_error, OCPtr};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ImageSubRange {
    crate mip_levels: ops::Range<u32>,
    crate layers: ops::Range<u32>,
}

/// Implementation of `ImageBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct ImageBuilder {
    metal_device: OCPtr<metal::MTLDevice>,
    extents: Option<ImageExtents>,
    num_layers: Option<u32>,
    num_mip_levels: u32,
    format: Option<base::ImageFormat>,
    usage: base::ImageUsageFlags,
    label: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum ImageExtents {
    OneD(u32),
    TwoD(u32, u32),
    ThreeD(u32, u32, u32),
    Cube(u32),
}

zangfx_impl_object! { ImageBuilder: dyn base::ImageBuilder, dyn crate::Debug, dyn base::SetLabel }

unsafe impl Send for ImageBuilder {}
unsafe impl Sync for ImageBuilder {}

impl ImageBuilder {
    /// Construct a `ImageBuilder`.
    ///
    /// It's up to the caller to make sure `metal_device` is valid.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device: OCPtr::new(metal_device).expect("nil device"),
            extents: None,
            num_layers: None,
            num_mip_levels: 1,
            format: None,
            usage: base::ImageUsageFlags::default(),
            label: None,
        }
    }
}

impl base::SetLabel for ImageBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
    }
}

impl base::ImageBuilder for ImageBuilder {
    fn queue(&mut self, _queue: &base::CmdQueueRef) -> &mut dyn base::ImageBuilder {
        self
    }

    fn extents(&mut self, v: &[u32]) -> &mut dyn base::ImageBuilder {
        self.extents = Some(match v.len() {
            1 => ImageExtents::OneD(v[0]),
            2 => ImageExtents::TwoD(v[0], v[1]),
            3 => ImageExtents::ThreeD(v[0], v[1], v[2]),
            _ => panic!("Invalid number of elements"),
        });
        self
    }

    fn extents_cube(&mut self, v: u32) -> &mut dyn base::ImageBuilder {
        self.extents = Some(ImageExtents::Cube(v));
        self
    }

    fn num_layers(&mut self, v: Option<u32>) -> &mut dyn base::ImageBuilder {
        self.num_layers = v;
        self
    }

    fn num_mip_levels(&mut self, v: u32) -> &mut dyn base::ImageBuilder {
        self.num_mip_levels = v;
        self
    }

    fn format(&mut self, v: base::ImageFormat) -> &mut dyn base::ImageBuilder {
        self.format = Some(v);
        self
    }

    fn usage(&mut self, v: base::ImageUsageFlags) -> &mut dyn base::ImageBuilder {
        self.usage = v;
        self
    }

    fn build(&mut self) -> Result<base::ImageRef> {
        let extents = self.extents.expect("extents");

        let format = self.format.expect("format");

        let metal_desc =
            unsafe { OCPtr::from_raw(metal::MTLTextureDescriptor::alloc().init()).unwrap() };

        use zangfx_metal_rs::MTLTextureType::{Cube, CubeArray, D1Array, D2Array, D1, D2, D3};
        let (ty, dims) = match (extents, self.num_layers) {
            (ImageExtents::OneD(x), None) => (D1, [x, 1, 1]),
            (ImageExtents::OneD(x), Some(_)) => (D1Array, [x, 1, 1]),
            (ImageExtents::TwoD(x, y), None) => (D2, [x, y, 1]),
            (ImageExtents::TwoD(x, y), Some(_)) => (D2Array, [x, y, 1]),
            (ImageExtents::ThreeD(x, y, z), None) => (D3, [x, y, z]),
            (ImageExtents::Cube(x), None) => (Cube, [x, x, 1]),
            (ImageExtents::Cube(x), Some(_)) => (CubeArray, [x, x, 1]),
            _ => panic!("unsupported image type"),
        };

        metal_desc.set_texture_type(ty);

        let mut usage = metal::MTLTextureUsage::empty();
        if self
            .usage
            .intersects(flags![base::ImageUsageFlags::{Sampled | Storage}])
        {
            usage |= metal::MTLTextureUsageShaderRead;
        }
        if self.usage.intersects(base::ImageUsageFlags::Storage) {
            usage |= metal::MTLTextureUsageShaderWrite;
        }
        if self.usage.intersects(base::ImageUsageFlags::Render) {
            usage |= metal::MTLTextureUsageRenderTarget;
        }
        if self
            .usage
            .intersects(flags![base::ImageUsageFlags::{MutableType | MutableFormat | PartialView}])
        {
            usage |= metal::MTLTextureUsagePixelFormatView;
        }
        metal_desc.set_usage(usage);

        // ZanGFX does not allow host-visible images
        metal_desc.set_resource_options(
            metal::MTLResourceStorageModePrivate | metal::MTLResourceHazardTrackingModeUntracked,
        );
        metal_desc.set_storage_mode(metal::MTLStorageMode::Private);

        let metal_format = translate_image_format(format).expect("Unsupported image format");
        metal_desc.set_pixel_format(metal_format);

        metal_desc.set_width(dims[0] as u64);
        metal_desc.set_height(dims[1] as u64);
        metal_desc.set_depth(dims[2] as u64);

        metal_desc.set_mipmap_level_count(self.num_mip_levels as u64);
        metal_desc.set_sample_count(1);
        metal_desc.set_array_length(self.num_layers.unwrap_or(1) as u64);

        let num_bytes_per_pixel = format.size_class().num_bytes_per_pixel();

        Ok(Image::new(
            *self.metal_device,
            metal_desc,
            num_bytes_per_pixel,
            self.label.clone(),
        )
        .into())
    }
}

/// Implementation of `Image` for Metal.
#[derive(Debug, Clone)]
pub struct Image {
    data: Arc<UnsafeCell<ImageData>>,
}

zangfx_impl_handle! { Image, base::ImageRef }

unsafe impl Send for Image {}
unsafe impl Sync for Image {}

#[derive(Debug)]
struct ImageData {
    metal_desc: Option<OCPtr<metal::MTLTextureDescriptor>>,
    metal_texture: Option<OCPtr<metal::MTLTexture>>,
    num_bytes_per_pixel: usize,
    memory_req: Option<base::MemoryReq>,
    label: Option<String>,
}

impl Image {
    fn new(
        metal_device: metal::MTLDevice,
        metal_desc: OCPtr<metal::MTLTextureDescriptor>,
        num_bytes_per_pixel: usize,
        label: Option<String>,
    ) -> Self {
        let metal_req = metal_device.heap_texture_size_and_align_with_descriptor(*metal_desc);
        let memory_req = base::MemoryReq {
            size: metal_req.size,
            align: metal_req.align,
            memory_types: 1 << crate::MEMORY_TYPE_PRIVATE,
        };

        let data = ImageData {
            metal_desc: Some(metal_desc),
            metal_texture: None,
            num_bytes_per_pixel,
            memory_req: Some(memory_req),
            label,
        };

        Self {
            data: Arc::new(UnsafeCell::new(data)),
        }
    }

    /// Construct a `Image` from a given raw `MTLTexture`.
    ///
    /// - The constructed `Image` will be initally in the Allocated state.
    /// - The constructed `Image` does not support `Image::get_memory_req`.
    pub unsafe fn from_raw(metal_texture: metal::MTLTexture) -> Self {
        let metal_format = metal_texture.pixel_format();
        let format = translate_metal_pixel_format(metal_format);

        let data = ImageData {
            metal_desc: None,
            metal_texture: OCPtr::from_raw(metal_texture),
            label: None,
            memory_req: None,
            num_bytes_per_pixel: format.size_class().num_bytes_per_pixel(),
        };

        Self {
            data: Arc::new(UnsafeCell::new(data)),
        }
    }

    unsafe fn data(&self) -> &mut ImageData {
        &mut *self.data.get()
    }

    /// Return the underlying `MTLTexture`. Returns `nil` for `Image`s in the
    /// Prototype state (i.e., not allocated on a heap).
    pub fn metal_texture(&self) -> metal::MTLTexture {
        unsafe {
            if let Some(ref p) = self.data().metal_texture {
                **p
            } else {
                metal::MTLTexture::nil()
            }
        }
    }

    pub(super) fn prototype_metal_desc(&self) -> metal::MTLTextureDescriptor {
        unsafe { **self.data().metal_desc.as_ref().expect("not prototype") }
    }

    pub(super) fn materialize(&self, metal_texture: OCPtr<metal::MTLTexture>) {
        let data = unsafe { self.data() };
        assert!(data.metal_texture.is_none(), "already materialized");
        data.metal_texture = Some(metal_texture);

        if let Some(label) = data.label.take() {
            data.metal_texture.as_ref().unwrap().set_label(&label);
        }

        // We don't need it anymore
        data.metal_desc = None;
    }

    pub(super) fn num_bytes_per_pixel(&self) -> usize {
        unsafe { self.data() }.num_bytes_per_pixel
    }

    pub(super) fn resolve_subrange(&self, range: &base::ImageSubRange) -> ImageSubRange {
        let metal_texture = self.metal_texture();
        debug_assert!(!metal_texture.is_null());
        ImageSubRange {
            mip_levels: range
                .mip_levels
                .clone()
                .unwrap_or_else(|| 0..metal_texture.mipmap_level_count() as u32),
            layers: range
                .layers
                .clone()
                .unwrap_or_else(|| 0..metal_texture.array_length() as u32),
        }
    }
}

impl base::Image for Image {
    fn build_image_view(&self) -> base::ImageViewBuilderRef {
        Box::new(ImageViewBuilder::new(self.clone()))
    }

    fn get_memory_req(&self) -> Result<base::MemoryReq> {
        Ok(unsafe { self.data() }
            .memory_req
            .expect("This image does not support get_memory_req"))
    }
}

/// Implementation of `ImageViewBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct ImageViewBuilder {
    image: Image,
    subrange: base::ImageSubRange,
    format: Option<base::ImageFormat>,
    image_type: Option<base::ImageType>,
}

zangfx_impl_object! { ImageViewBuilder: dyn base::ImageViewBuilder, dyn crate::Debug }

impl ImageViewBuilder {
    /// Construct a `ImageViewBuilder`.
    pub fn new(image: Image) -> Self {
        Self {
            image,
            subrange: Default::default(),
            format: None,
            image_type: None,
        }
    }
}

impl base::ImageViewBuilder for ImageViewBuilder {
    fn subrange(&mut self, v: &base::ImageSubRange) -> &mut dyn base::ImageViewBuilder {
        self.subrange = v.clone();
        self
    }

    fn format(&mut self, v: base::ImageFormat) -> &mut dyn base::ImageViewBuilder {
        self.format = Some(v);
        self
    }

    fn image_type(&mut self, v: base::ImageType) -> &mut dyn base::ImageViewBuilder {
        self.image_type = Some(v);
        self
    }

    fn build(&mut self) -> Result<base::ImageRef> {
        let ref image = self.image;
        let metal_texture = image.metal_texture();
        assert!(!metal_texture.is_null());

        let num_bytes_per_pixel = image.num_bytes_per_pixel();

        let subrange = image.resolve_subrange(&self.subrange);
        let full_subrange = image.resolve_subrange(&Default::default());
        let metal_format = self
            .format
            .map(|x| translate_image_format(x).expect("Unsupported image format"))
            .unwrap_or_else(|| metal_texture.pixel_format());

        use zangfx_metal_rs::MTLTextureType::{Cube, CubeArray, D2Array, D1, D2, D3};
        let metal_ty = self
            .image_type
            .map(|ty| match ty {
                base::ImageType::OneD => D1,
                base::ImageType::TwoD => D2,
                base::ImageType::TwoDArray => D2Array,
                base::ImageType::ThreeD => D3,
                base::ImageType::Cube => Cube,
                base::ImageType::CubeArray => CubeArray,
            })
            .unwrap_or_else(|| metal_texture.texture_type());

        let new_metal_texture;

        if subrange == full_subrange
            && metal_format == metal_texture.pixel_format()
            && metal_ty == metal_texture.texture_type()
        {
            new_metal_texture = metal_texture.clone();
        } else {
            new_metal_texture = metal_texture.new_texture_view_from_slice(
                metal_format,
                metal_ty,
                NSRange::new(
                    subrange.mip_levels.start as u64,
                    (subrange.mip_levels.end - subrange.mip_levels.start) as u64,
                ),
                NSRange::new(
                    subrange.layers.start as u64,
                    (subrange.layers.end - subrange.layers.start) as u64,
                ),
            );

            if new_metal_texture.is_null() {
                return Err(nil_error(
                    "MTLTexture newTextureViewWithPixelFormat:textureType:levels:slices:",
                ));
            }
        }

        let data = ImageData {
            metal_desc: None,
            metal_texture: Some(unsafe { OCPtr::from_raw(new_metal_texture) }.unwrap()),
            num_bytes_per_pixel,
            memory_req: None,
            label: None,
        };

        Ok(Image {
            data: Arc::new(UnsafeCell::new(data)),
        }
        .into())
    }
}
