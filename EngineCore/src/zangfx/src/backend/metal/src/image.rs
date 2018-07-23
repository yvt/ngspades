//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Image` for Metal.
use std::ops;
use zangfx_base::{self as base, Result};
use zangfx_base::{zangfx_impl_object, interfaces, vtable_for, zangfx_impl_handle};
use metal;
use cocoa::foundation::NSRange;
use ngsenumflags::flags;

use crate::utils::{nil_error, OCPtr};
use crate::formats::{translate_image_format, translate_metal_pixel_format};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ImageSubRange {
    pub mip_levels: ops::Range<u32>,
    pub layers: ops::Range<u32>,
}

/// Implementation of `ImageBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct ImageBuilder {
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

zangfx_impl_object! { ImageBuilder: base::ImageBuilder, crate::Debug, base::SetLabel }

impl ImageBuilder {
    /// Construct a `ImageBuilder`.
    pub fn new() -> Self {
        Self {
            extents: None,
            num_layers: None,
            num_mip_levels: 1,
            format: None,
            usage: base::ImageUsage::default_flags(),
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
    fn queue(&mut self, _queue: &base::CmdQueueRef) -> &mut base::ImageBuilder {
        self
    }

    fn extents(&mut self, v: &[u32]) -> &mut base::ImageBuilder {
        self.extents = Some(match v.len() {
            1 => ImageExtents::OneD(v[0]),
            2 => ImageExtents::TwoD(v[0], v[1]),
            3 => ImageExtents::ThreeD(v[0], v[1], v[2]),
            _ => panic!("Invalid number of elements"),
        });
        self
    }

    fn extents_cube(&mut self, v: u32) -> &mut base::ImageBuilder {
        self.extents = Some(ImageExtents::Cube(v));
        self
    }

    fn num_layers(&mut self, v: Option<u32>) -> &mut base::ImageBuilder {
        self.num_layers = v;
        self
    }

    fn num_mip_levels(&mut self, v: u32) -> &mut base::ImageBuilder {
        self.num_mip_levels = v;
        self
    }

    fn format(&mut self, v: base::ImageFormat) -> &mut base::ImageBuilder {
        self.format = Some(v);
        self
    }

    fn usage(&mut self, v: base::ImageUsageFlags) -> &mut base::ImageBuilder {
        self.usage = v;
        self
    }

    fn build(&mut self) -> Result<base::ImageRef> {
        let extents = self.extents
            .expect("extents");

        let format = self.format
            .expect("format");

        let metal_desc =
            unsafe { OCPtr::from_raw(metal::MTLTextureDescriptor::alloc().init()).unwrap() };

        use crate::metal::MTLTextureType::*;
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
        if self.usage
            .intersects(flags![base::ImageUsage::{Sampled | Storage}])
        {
            usage |= metal::MTLTextureUsageShaderRead;
        }
        if self.usage.intersects(base::ImageUsage::Storage) {
            usage |= metal::MTLTextureUsageShaderWrite;
        }
        if self.usage.intersects(base::ImageUsage::Render) {
            usage |= metal::MTLTextureUsageRenderTarget;
        }
        if self.usage
            .intersects(flags![base::ImageUsage::{MutableType | MutableFormat | PartialView}])
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
            metal_desc,
            num_bytes_per_pixel,
            self.label.clone(),
        ).into())
    }
}

/// Implementation of `Image` for Metal.
#[derive(Debug, Clone)]
pub struct Image {
    data: *mut ImageData,
}

zangfx_impl_handle! { Image, base::ImageRef }

unsafe impl Send for Image {}
unsafe impl Sync for Image {}

#[derive(Debug)]
struct ImageData {
    metal_desc: Option<OCPtr<metal::MTLTextureDescriptor>>,
    metal_texture: Option<OCPtr<metal::MTLTexture>>,
    num_bytes_per_pixel: usize,
    label: Option<String>,
}

impl Image {
    fn new(
        metal_desc: OCPtr<metal::MTLTextureDescriptor>,
        num_bytes_per_pixel: usize,
        label: Option<String>,
    ) -> Self {
        let data = ImageData {
            metal_desc: Some(metal_desc),
            metal_texture: None,
            num_bytes_per_pixel,
            label,
        };

        Self {
            data: Box::into_raw(Box::new(data)),
        }
    }

    /// Construct a `Image` from a given raw `MTLTexture`.
    ///
    /// The constructed `Image` will be initally in the Allocated state.
    pub unsafe fn from_raw(metal_texture: metal::MTLTexture) -> Self {
        let metal_format = metal_texture.pixel_format();
        let format = translate_metal_pixel_format(metal_format);

        let data = ImageData {
            metal_desc: None,
            metal_texture: OCPtr::from_raw(metal_texture),
            label: None,
            num_bytes_per_pixel: format.size_class().num_bytes_per_pixel(),
        };

        Self {
            data: Box::into_raw(Box::new(data)),
        }
    }

    unsafe fn data(&self) -> &mut ImageData {
        &mut *self.data
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
        unsafe { **self.data().metal_desc.as_ref().unwrap() }
    }

    pub(super) fn materialize(&self, metal_texture: OCPtr<metal::MTLTexture>) {
        let data = unsafe { self.data() };
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

    pub(super) fn memory_req(&self, metal_device: metal::MTLDevice) -> base::MemoryReq {
        let metal_req =
            metal_device.heap_texture_size_and_align_with_descriptor(self.prototype_metal_desc());
        base::MemoryReq {
            size: metal_req.size,
            align: metal_req.align,
            memory_types: 1 << crate::MEMORY_TYPE_PRIVATE,
        }
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

    pub(super) unsafe fn destroy(&self) {
        Box::from_raw(self.data);
    }
}

impl base::Image for Image {
    fn build_image_view(&self) -> base::ImageViewBuilderRef {
        unimplemented!() // Box::new(image::ImageViewBuilder::new())
    }

    fn get_memory_req(&self) -> Result<base::MemoryReq> {
        unimplemented!()
    }
}

/// Implementation of `ImageViewBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct ImageViewBuilder {
    image: Option<Image>,
    subrange: base::ImageSubRange,
    format: Option<base::ImageFormat>,
    image_type: Option<base::ImageType>,
}

zangfx_impl_object! { ImageViewBuilder: base::ImageViewBuilder, crate::Debug }

impl ImageViewBuilder {
    /// Construct a `ImageBuilder`.
    pub fn new() -> Self {
        Self {
            image: None,
            subrange: Default::default(),
            format: None,
            image_type: None,
        }
    }
}

impl base::ImageViewBuilder for ImageViewBuilder {
    /* fn image(&mut self, v: &base::Image) -> &mut base::ImageViewBuilder {
        let my_image: &Image = v.downcast_ref().expect("bad image type");
        self.image = Some(my_image.clone());
        self
    } */

    fn subrange(&mut self, v: &base::ImageSubRange) -> &mut base::ImageViewBuilder {
        self.subrange = v.clone();
        self
    }

    fn format(&mut self, v: base::ImageFormat) -> &mut base::ImageViewBuilder {
        self.format = Some(v);
        self
    }

    fn image_type(&mut self, v: base::ImageType) -> &mut base::ImageViewBuilder {
        self.image_type = Some(v);
        self
    }

    fn build(&mut self) -> Result<base::ImageRef> {
        let image = self.image
            .as_ref()
            .expect("image");
        let metal_texture = image.metal_texture();
        assert!(!metal_texture.is_null());

        let subrange = image.resolve_subrange(&self.subrange);
        let full_subrange = image.resolve_subrange(&Default::default());
        let metal_format = self.format
            .map(|x| translate_image_format(x).expect("Unsupported image format"))
            .unwrap_or_else(|| metal_texture.pixel_format());

        use crate::metal::MTLTextureType::*;
        let metal_ty = self.image_type
            .map(|ty| match ty {
                base::ImageType::OneD => D1,
                base::ImageType::TwoD => D2,
                base::ImageType::TwoDArray => D2Array,
                base::ImageType::ThreeD => D3,
                base::ImageType::Cube => Cube,
                base::ImageType::CubeArray => CubeArray,
            })
            .unwrap_or_else(|| metal_texture.texture_type());

        if subrange == full_subrange && metal_format == metal_texture.pixel_format()
            && metal_ty == metal_texture.texture_type()
        {
            unimplemented!()
            // return Ok(base::ImageView::new(ImageView::new(metal_texture, false)));
        }

        let view = metal_texture.new_texture_view_from_slice(
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

        if view.is_null() {
            return Err(nil_error(
                "MTLTexture newTextureViewWithPixelFormat:textureType:levels:slices:",
            ));
        }

        unimplemented!()
        // Ok(base::ImageView::new(ImageView::new(metal_texture, true)))
    }
}
