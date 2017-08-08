//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core::{self, Validate};
use metal;

use cgmath::Vector3;
use cocoa::foundation::NSRange;

use {OCPtr, RefEqArc};
use imp::{translate_image_format, translate_metal_pixel_format};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Image {
    data: RefEqArc<ImageData>,
}

#[derive(Debug)]
struct ImageData {
    metal_texture: OCPtr<metal::MTLTexture>,
    can_make_views: bool,
    desc: core::ImageDescription,
    num_bytes_per_pixel: usize,
}

impl core::Image for Image {}

impl core::Marker for Image {
    fn set_label(&self, label: Option<&str>) {
        self.data.metal_texture.set_label(label.unwrap_or(""));
    }
}

unsafe impl Send for ImageData {}
unsafe impl Sync for ImageData {} // no interior mutability

impl Image {
    pub(crate) fn new(
        desc: &core::ImageDescription,
        metal_device: metal::MTLDevice,
    ) -> core::Result<Image> {
        if desc.tiling == core::ImageTiling::Linear {
            unimplemented!();
        }

        let metal_desc =
            unsafe { OCPtr::from_raw(metal::MTLTextureDescriptor::alloc().init()).unwrap() };

        let can_make_views = desc.flags.intersects(
            core::ImageFlag::MutableType | core::ImageFlag::MutableFormat |
                core::ImageFlag::SubrangeViewCompatible,
        );

        let texture_type = match desc.image_type {
            core::ImageType::OneD => metal::MTLTextureType::D1,
            core::ImageType::TwoD => metal::MTLTextureType::D2,
            core::ImageType::TwoDArray => metal::MTLTextureType::D2Array,
            core::ImageType::Cube => metal::MTLTextureType::Cube,
            core::ImageType::CubeArray => metal::MTLTextureType::CubeArray,
            core::ImageType::ThreeD => metal::MTLTextureType::D3,
        };
        metal_desc.set_texture_type(texture_type);

        let mut usage = metal::MTLTextureUsageUnknown;
        if desc.usage.contains(
            core::ImageUsage::InputAttachment | core::ImageUsage::Sampled |
                core::ImageUsage::Storage,
        )
        {
            usage |= metal::MTLTextureUsageShaderRead;
        }
        if desc.usage.contains(core::ImageUsage::Storage) {
            usage |= metal::MTLTextureUsageShaderWrite;
        }
        if desc.usage.contains(
            core::ImageUsage::ColorAttachment | core::ImageUsage::DepthStencilAttachment,
        )
        {
            usage |= metal::MTLTextureUsageRenderTarget;
        }
        if can_make_views {
            usage |= metal::MTLTextureUsagePixelFormatView;
        }
        metal_desc.set_usage(usage);

        let options: metal::MTLResourceOptions = match desc.storage_mode {
            core::StorageMode::Private => metal::MTLResourceStorageModePrivate,
            core::StorageMode::Shared => metal::MTLResourceStorageModeShared,
            core::StorageMode::Memoryless => unimplemented!(),
        };
        metal_desc.set_resource_options(options);

        // do I really have to specify this twice?
        let storage_mode = match desc.storage_mode {
            core::StorageMode::Private => metal::MTLStorageMode::Private,
            core::StorageMode::Shared => metal::MTLStorageMode::Shared,
            core::StorageMode::Memoryless => unimplemented!(),
        };
        metal_desc.set_storage_mode(storage_mode);

        let format = translate_image_format(desc.format).expect("Unsupported image format");
        metal_desc.set_pixel_format(format);

        metal_desc.set_width(desc.extent.x as u64);
        metal_desc.set_height(desc.extent.y as u64);
        metal_desc.set_depth(desc.extent.z as u64);
        metal_desc.set_mipmap_level_count(desc.num_mip_levels as u64);
        metal_desc.set_sample_count(1);
        metal_desc.set_array_length(desc.num_array_layers as u64);

        // TODO: handle allocation failure
        let metal_texture =
            unsafe { OCPtr::from_raw(metal_device.new_texture(*metal_desc)) }.expect("texture creation failed");

        Ok(Image {
            data: RefEqArc::new(ImageData {
                metal_texture,
                can_make_views,
                desc: desc.clone(),
                num_bytes_per_pixel: desc.format.size_class().num_bytes_per_pixel(),
            }),
        })
    }

    pub fn from_raw(raw: metal::MTLTexture) -> Self {
        assert!(!raw.is_null());
        let metal_usage = raw.usage();
        let can_make_views = metal_usage.contains(metal::MTLTextureUsagePixelFormatView);

        let metal_format = raw.pixel_format();
        let format = translate_metal_pixel_format(metal_format);

        let mut usage = core::ImageUsageFlags::empty();
        if metal_usage.contains(metal::MTLTextureUsageShaderRead) {
            usage.insert(core::ImageUsage::Sampled.into());
            usage.insert(core::ImageUsage::InputAttachment.into());
        }
        if metal_usage.contains(metal::MTLTextureUsageShaderWrite) {
            usage.insert(core::ImageUsage::Storage.into());
        }
        if metal_usage.contains(metal::MTLTextureUsageRenderTarget) {
            if format.has_color() {
                usage.insert(core::ImageUsage::ColorAttachment.into());
            }
            if format.has_depth() || format.has_stencil() {
                usage.insert(core::ImageUsage::DepthStencilAttachment.into());
            }
        }

        let storage_mode = match raw.storage_mode() {
            metal::MTLStorageMode::Private => core::StorageMode::Private,
            metal::MTLStorageMode::Managed => core::StorageMode::Private,
            metal::MTLStorageMode::Shared => core::StorageMode::Shared,
        };

        let desc = core::ImageDescription {
            flags: if can_make_views {
                core::ImageFlag::MutableFormat | core::ImageFlag::SubrangeViewCompatible |
                    core::ImageFlag::MutableType
            } else {
                core::ImageFlags::empty()
            },
            usage,
            image_type: core::ImageType::TwoD, // TODO: translate image type
            format,
            extent: Vector3::new(raw.width() as u32, raw.height() as u32, raw.depth() as u32),
            num_mip_levels: raw.mipmap_level_count() as u32,
            num_array_layers: raw.array_length() as u32,
            initial_layout: core::ImageLayout::Undefined,
            tiling: core::ImageTiling::Optimal, // TODO: guess tiling
            storage_mode,
        };
        Self {
            data: RefEqArc::new(ImageData {
                metal_texture: OCPtr::new(raw).unwrap(),
                can_make_views,
                desc,
                num_bytes_per_pixel: format.size_class().num_bytes_per_pixel(),
            }),
        }
    }

    pub fn metal_texture(&self) -> metal::MTLTexture {
        *self.data.metal_texture
    }

    pub(crate) fn num_bytes_per_pixel(&self) -> usize {
        self.data.num_bytes_per_pixel
    }

    pub(crate) fn resolve_subresource_range(
        &self,
        range: &core::ImageSubresourceRange,
    ) -> ImageSubresourceRange {
        let ref desc = self.data.desc;
        ImageSubresourceRange {
            base_mip_level: range.base_mip_level,
            base_array_layer: range.base_array_layer,
            num_mip_levels: range.num_mip_levels.unwrap_or_else(|| {
                desc.num_mip_levels - range.base_mip_level
            }),
            num_array_layers: range.num_array_layers.unwrap_or_else(|| {
                desc.num_array_layers - range.base_array_layer
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageView {
    data: RefEqArc<ImageViewData>,
}

#[derive(Debug)]
struct ImageViewData {
    metal_texture: OCPtr<metal::MTLTexture>,

    /// Indicates whether `metal_texture` is already subranged by
    /// `range`. It also becomes `true` if `ImageView` is not subranged
    /// (i.e. entire the image is specified) at all.
    metal_texture_subranged: bool,

    /// Specifies which part of the `metal_texture` should be used as
    /// the contents of `ImageView`.
    range: ImageSubresourceRange,
}

#[derive(Debug)]
pub(crate) struct ImageSubresourceRange {
    pub base_mip_level: u32,
    pub num_mip_levels: u32,
    pub base_array_layer: u32,
    pub num_array_layers: u32,
}

impl core::Marker for ImageView {
    fn set_label(&self, label: Option<&str>) {
        self.data.metal_texture.set_label(label.unwrap_or(""));
    }
}

impl core::ImageView for ImageView {}

unsafe impl Send for ImageViewData {}
unsafe impl Sync for ImageViewData {} // no interior mutability

impl ImageView {
    /// Constructs a new `ImageView` with a given `MTLTexture`.
    ///
    /// `raw` must not be null. Otherwise, a panic will occur.
    ///
    /// The returned `ImageView` retains a reference to the given `MTLTexture`.
    pub fn new(raw: metal::MTLTexture) -> Self {
        Self {
            data: RefEqArc::new(ImageViewData {
                metal_texture: OCPtr::new(raw).unwrap(),
                metal_texture_subranged: true,
                range: ImageSubresourceRange {
                    base_mip_level: 0,
                    num_mip_levels: raw.mipmap_level_count() as u32,
                    base_array_layer: 0,
                    num_array_layers: raw.array_length() as u32,
                },
            }),
        }
    }

    pub(crate) fn new_from_description(
        desc: &core::ImageViewDescription<Image>,
        cap: &core::DeviceCapabilities,
    ) -> core::Result<Self> {
        let image = desc.image;

        // the original image description is inaccessible from `factory.rs`, so...
        core::CombinedImageAndImageViewDescription(&image.data.desc, desc)
            .debug_expect_valid(Some(cap), "");

        let range = image.resolve_subresource_range(&desc.range);

        let subranged = range.base_mip_level != 0 || range.base_array_layer != 0 ||
            range.num_mip_levels != image.data.desc.num_mip_levels ||
            range.num_array_layers != image.data.desc.num_array_layers;

        let data;

        if desc.image_type == image.data.desc.image_type && desc.format == image.data.desc.format &&
            (!subranged || !image.data.can_make_views)
        {
            data = ImageViewData {
                metal_texture: image.data.metal_texture.clone(),
                metal_texture_subranged: subranged,
                range,
            };
        } else {
            let format = translate_image_format(desc.format).expect("Unsupported image format");
            let texture_type = match desc.image_type {
                core::ImageType::OneD => metal::MTLTextureType::D1,
                core::ImageType::TwoD => metal::MTLTextureType::D2,
                core::ImageType::TwoDArray => metal::MTLTextureType::D2Array,
                core::ImageType::Cube => metal::MTLTextureType::Cube,
                core::ImageType::CubeArray => metal::MTLTextureType::CubeArray,
                core::ImageType::ThreeD => metal::MTLTextureType::D3,
            };
            let view = image.data.metal_texture.new_texture_view_from_slice(
                format,
                texture_type,
                NSRange::new(
                    range.base_mip_level as u64,
                    range.num_mip_levels as u64,
                ),
                NSRange::new(
                    range.base_array_layer as u64,
                    range.num_array_layers as u64,
                ),
            );
            data = ImageViewData {
                metal_texture: OCPtr::new(view).unwrap(),
                metal_texture_subranged: false,
                range: ImageSubresourceRange {
                    // relative offset from `ImageViewData::metal_texture`
                    base_mip_level: 0,
                    base_array_layer: 0,
                    ..range
                },
            };
        }

        Ok(ImageView { data: RefEqArc::new(data) })
    }

    /// Return `MTLTexture`.
    ///
    /// Panics
    /// ======
    ///
    /// Might panic if the image view does not specify the all mip levels
    /// and all array layers, and `ImageFlag::SubrangeViewCompatible` was not specified
    /// when the image was created (in which case a flag necessary to create
    /// texture views is not specified when the original `MTLTexture` is created, so
    /// `ImageView` cannot return a `MTLTexture` that only contains the range
    /// specified by `ImageSubresourceRange`).
    pub fn metal_texture(&self) -> metal::MTLTexture {
        assert!(
            !self.data.metal_texture_subranged,
            "Inappropriate usage of ImageView -- check ImageFlags"
        );
        *self.data.metal_texture
    }

    /// Return `MTLTexture` and `ImageSubresourceRange`.
    /// `ImageSubresourceRange` specifies which part of the returned `MTLTexture`
    /// should be used as the contents of `ImageView`.
    pub(crate) fn metal_texture_with_range(&self) -> (metal::MTLTexture, &ImageSubresourceRange) {
        (*self.data.metal_texture, &self.data.range)
    }
}
