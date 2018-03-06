//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Image` for Metal.
use base;
use common::{Error, ErrorKind, Result};
use metal;

use utils::OCPtr;
use formats::translate_image_format;

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

zangfx_impl_object! { ImageBuilder: base::ImageBuilder, ::Debug }

impl ImageBuilder {
    /// Construct a `ImageBuilder`.
    pub fn new() -> Self {
        Self {
            extents: None,
            num_layers: None,
            num_mip_levels: 1,
            format: None,
            usage: flags![base::ImageUsage::{CopyWrite | Sampled}],
            label: None,
        }
    }
}

impl base::ImageBuilder for ImageBuilder {
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

    fn build(&mut self) -> Result<base::Image> {
        let extents = self.extents
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "extents"))?;

        let format = self.format
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "format"))?;

        let metal_desc =
            unsafe { OCPtr::from_raw(metal::MTLTextureDescriptor::alloc().init()).unwrap() };

        use metal::MTLTextureType::*;
        let (ty, dims) = match (extents, self.num_layers) {
            (ImageExtents::OneD(x), None) => (D1, [x, 1, 1]),
            (ImageExtents::OneD(x), Some(_)) => (D1Array, [x, 1, 1]),
            (ImageExtents::TwoD(x, y), None) => (D2, [x, y, 1]),
            (ImageExtents::TwoD(x, y), Some(_)) => (D2Array, [x, y, 1]),
            (ImageExtents::ThreeD(x, y, z), None) => (D3, [x, y, z]),
            (ImageExtents::Cube(x), None) => (Cube, [x, x, 1]),
            (ImageExtents::Cube(x), Some(_)) => (CubeArray, [x, x, 1]),
            _ => {
                return Err(Error::with_detail(
                    ErrorKind::InvalidUsage,
                    "unsupported image type",
                ))
            }
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

        let format = translate_image_format(format).expect("Unsupported image format");
        metal_desc.set_pixel_format(format);

        metal_desc.set_width(dims[0] as u64);
        metal_desc.set_height(dims[1] as u64);
        metal_desc.set_depth(dims[2] as u64);

        metal_desc.set_mipmap_level_count(self.num_mip_levels as u64);
        metal_desc.set_sample_count(1);
        metal_desc.set_array_length(self.num_layers.unwrap_or(1) as u64);

        Ok(base::Image::new(Image::new(metal_desc, self.label.clone())))
    }
}

/// Implementation of `Image` for Metal.
#[derive(Debug, Clone)]
pub struct Image {
    data: *mut ImageData,
}

zangfx_impl_handle! { Image, base::Image }

unsafe impl Send for Image {}
unsafe impl Sync for Image {}

#[derive(Debug)]
struct ImageData {
    metal_desc: Option<OCPtr<metal::MTLTextureDescriptor>>,
    metal_texture: Option<OCPtr<metal::MTLTexture>>,
    label: Option<String>,
}

impl Image {
    fn new(metal_desc: OCPtr<metal::MTLTextureDescriptor>, label: Option<String>) -> Self {
        let data = ImageData {
            metal_desc: Some(metal_desc),
            metal_texture: None,
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
        let data = ImageData {
            metal_desc: None,
            metal_texture: OCPtr::from_raw(metal_texture),
            label: None,
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

    pub(super) fn memory_req(&self, metal_device: metal::MTLDevice) -> base::MemoryReq {
        let metal_req =
            metal_device.heap_texture_size_and_align_with_descriptor(self.prototype_metal_desc());
        base::MemoryReq {
            size: metal_req.size,
            align: metal_req.align,
            memory_types: 1 << ::MEMORY_TYPE_PRIVATE,
        }
    }

    pub(super) unsafe fn destroy(&self) {
        Box::from_raw(self.data);
    }
}
