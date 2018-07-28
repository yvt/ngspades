//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Image` for Vulkan.
use ash::version::*;
use ash::vk;
use std::mem::transmute;

use zangfx_base as base;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_handle, zangfx_impl_object};
use zangfx_base::{Error, ErrorKind, Result};
use zangfx_common::BinaryInteger;

use crate::device::DeviceRef;
use crate::formats::translate_image_format;
use crate::utils::{
    translate_generic_error_unwrap, translate_image_layout, translate_image_subresource_range,
};

/// Implementation of `ImageBuilder` for Vulkan.
#[derive(Debug)]
pub struct ImageBuilder {
    device: DeviceRef,
    extents: Option<ImageExtents>,
    num_layers: Option<u32>,
    num_mip_levels: u32,
    format: Option<base::ImageFormat>,
    usage: base::ImageUsageFlags,
}

#[derive(Debug, Clone, Copy)]
enum ImageExtents {
    OneD(u32),
    TwoD(u32, u32),
    ThreeD(u32, u32, u32),
    Cube(u32),
}

zangfx_impl_object! { ImageBuilder: dyn base::ImageBuilder, dyn (crate::Debug) }

impl ImageBuilder {
    pub(super) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            extents: None,
            num_layers: None,
            num_mip_levels: 1,
            format: None,
            usage: base::ImageUsage::default_flags(),
        }
    }
}

impl base::ImageBuilder for ImageBuilder {
    fn queue(&mut self, queue: &base::CmdQueueRef) -> &mut dyn base::ImageBuilder {
        unimplemented!();
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

        use ash::vk::ImageViewType::*;
        let (image_view_type, dims) = match (extents, self.num_layers) {
            (ImageExtents::OneD(x), None) => (Type1d, [x, 1, 1]),
            (ImageExtents::OneD(x), Some(_)) => (Type1dArray, [x, 1, 1]),
            (ImageExtents::TwoD(x, y), None) => (Type2d, [x, y, 1]),
            (ImageExtents::TwoD(x, y), Some(_)) => (Type2dArray, [x, y, 1]),
            (ImageExtents::ThreeD(x, y, z), None) => (Type3d, [x, y, z]),
            (ImageExtents::Cube(x), None) => (Cube, [x, x, 1]),
            (ImageExtents::Cube(x), Some(_)) => (CubeArray, [x, x, 1]),
            _ => {
                panic!("unsupported image type");
            }
        };

        let mut flags = vk::ImageCreateFlags::empty();
        if self.usage.contains(base::ImageUsage::MutableFormat) {
            flags |= vk::IMAGE_CREATE_MUTABLE_FORMAT_BIT;
        }
        if let ImageExtents::Cube(_) = extents {
            // note: NgsGFX does not allow creating cube image views from
            // other kinds of images
            flags |= vk::IMAGE_CREATE_CUBE_COMPATIBLE_BIT;
        }

        let image_type = match image_view_type {
            Type1d | Type1dArray => vk::ImageType::Type1d,
            Type2d | Type2dArray | Cube | CubeArray => vk::ImageType::Type2d,
            Type3d => vk::ImageType::Type3d,
        };

        let mut array_layers = self.num_layers.unwrap_or(1);
        if let ImageExtents::Cube(_) = extents {
            array_layers *= 6;
        }

        let usage = translate_image_usage(self.usage, format);

        let mut aspect = vk::ImageAspectFlags::empty();
        if format.has_color() {
            aspect |= vk::IMAGE_ASPECT_COLOR_BIT;
        }
        if format.has_depth() {
            aspect |= vk::IMAGE_ASPECT_DEPTH_BIT;
        }
        if format.has_stencil() {
            aspect |= vk::IMAGE_ASPECT_STENCIL_BIT;
        }

        let format = translate_image_format(format).expect("unsupported image format");

        let meta = ImageMeta::new(image_view_type, aspect, format);

        let info = vk::ImageCreateInfo {
            s_type: vk::StructureType::ImageCreateInfo,
            p_next: ::null(),
            flags,
            image_type,
            format,
            extent: vk::Extent3D {
                width: dims[0],
                height: dims[1],
                depth: dims[2],
            },
            mip_levels: self.num_mip_levels,
            array_layers,
            samples: vk::SAMPLE_COUNT_1_BIT,
            tiling: vk::ImageTiling::Optimal,
            usage,
            sharing_mode: vk::SharingMode::Exclusive,
            queue_family_index_count: 0, // ignored for `SharingMode::Exclusive`
            p_queue_family_indices: ::null(),
            initial_layout: vk::ImageLayout::Undefined,
        };

        let vk_device = self.device.vk_device();
        let vk_image =
            unsafe { vk_device.create_image(&info, None) }.map_err(translate_generic_error_unwrap)?;
        Ok(Image { vk_image, meta }.into())
    }
}

/// Implementation of `Image` for Vulkan.
#[derive(Debug, Clone)]
pub struct Image {
    vk_image: vk::Image,

    /// The image's metadata. Used as default values for creating image views.
    meta: ImageMeta,
}

zangfx_impl_handle! { Image, base::ImageRef }

unsafe impl Sync for Image {}
unsafe impl Send for Image {}

impl Image {
    pub unsafe fn from_raw(vk_image: vk::Image, meta: ImageMeta) -> Self {
        Self { vk_image, meta }
    }

    pub fn vk_image(&self) -> vk::Image {
        self.vk_image
    }

    pub(super) fn meta(&self) -> ImageMeta {
        self.meta
    }

    pub(super) unsafe fn destroy(&self, vk_device: &crate::AshDevice) {
        vk_device.destroy_image(self.vk_image, None);
    }
}

impl base::Image for Image {
    fn build_image_view(&self) -> base::ImageViewBuilderRef {
        unimplemented!()
        // unsafe { Box::new(image::ImageViewBuilder::new(self.new_device_ref())) }
    }

    fn make_proxy(&mut self, queue: &base::CmdQueueRef) -> base::ImageRef {
        unimplemented!()
    }

    fn get_memory_req(&self) -> Result<base::MemoryReq> {
        unimplemented!()
    }
}

fn translate_image_usage(
    value: base::ImageUsageFlags,
    format: base::ImageFormat,
) -> vk::ImageUsageFlags {
    let mut usage = vk::ImageUsageFlags::empty();
    if value.contains(base::ImageUsage::CopyRead) {
        usage |= vk::IMAGE_USAGE_TRANSFER_SRC_BIT;
    }
    if value.contains(base::ImageUsage::CopyWrite) {
        usage |= vk::IMAGE_USAGE_TRANSFER_DST_BIT;
    }
    if value.contains(base::ImageUsage::Sampled) {
        usage |= vk::IMAGE_USAGE_SAMPLED_BIT;
    }
    if value.contains(base::ImageUsage::Storage) {
        usage |= vk::IMAGE_USAGE_STORAGE_BIT;
    }
    if value.contains(base::ImageUsage::Render) {
        if format.has_depth() || format.has_stencil() {
            usage |= vk::IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT;
        } else {
            usage |= vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT;
        }
    }
    usage
}

/// Compact representation of image metadata. Stored in `Image` along with
/// the vulkan image handle.
///
/// # Bit Fields
///
///  - `[2:0]`: The default mage view type
///  - `[6:3]`: The image aspect flags valid for the image
///  - `[14:7]`: THe image format
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageMeta(u16);

impl ImageMeta {
    pub fn new(
        image_view_type: vk::ImageViewType,
        image_aspects: vk::ImageAspectFlags,
        format: vk::Format,
    ) -> Self {
        let mut bits = 0;
        bits |= image_view_type as u16;
        bits |= (image_aspects.flags() as u16) << 3;
        bits |= (format as u16) << 7;
        ImageMeta(bits)
    }

    pub fn image_view_type(&self) -> vk::ImageViewType {
        unsafe { transmute(self.0.extract_u32(0..3)) }
    }

    pub fn image_aspects(&self) -> vk::ImageAspectFlags {
        vk::ImageAspectFlags::from_flags(self.0.extract_u32(3..7)).unwrap()
    }

    pub fn format(&self) -> vk::Format {
        unsafe { transmute(self.0.extract_u32(7..15)) }
    }
}

/// Implementation of `ImageViewBuilder` for Vulkan.
#[derive(Debug)]
pub struct ImageViewBuilder {
    device: DeviceRef,
    image: Option<Image>,
    subrange: base::ImageSubRange,
    format: Option<base::ImageFormat>,
    image_type: Option<base::ImageType>,
}

zangfx_impl_object! { ImageViewBuilder: dyn base::ImageViewBuilder, dyn (crate::Debug) }

impl ImageViewBuilder {
    pub(super) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            image: None,
            subrange: Default::default(),
            format: None,
            image_type: None,
        }
    }
}

impl base::ImageViewBuilder for ImageViewBuilder {
    /* fn image(&mut self, v: &base::ImageRef) -> &mut base::ImageViewBuilder {
        let my_image: &Image = v.downcast_ref().expect("bad image type");
        self.image = Some(my_image.clone());
        self
    } */

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
        let image: &Image = self.image.as_ref().expect("image");

        let flags = vk::ImageViewCreateFlags::empty();
        // flags: "reserved for future use"

        let view_type = self
            .image_type
            .map(|t| match t {
                base::ImageType::OneD => vk::ImageViewType::Type1d,
                base::ImageType::TwoD => vk::ImageViewType::Type2d,
                base::ImageType::TwoDArray => vk::ImageViewType::Type2dArray,
                base::ImageType::ThreeD => vk::ImageViewType::Type3d,
                base::ImageType::Cube => vk::ImageViewType::Cube,
                base::ImageType::CubeArray => vk::ImageViewType::CubeArray,
            })
            .unwrap_or(image.meta().image_view_type());

        let format = self
            .format
            .map(|f| translate_image_format(f).expect("unsupported image format"))
            .unwrap_or(image.meta().format());

        let is_ds = image
            .meta()
            .image_aspects()
            .intersects(vk::IMAGE_ASPECT_DEPTH_BIT | vk::IMAGE_ASPECT_STENCIL_BIT);

        let meta = ImageViewMeta::new(
            /* translate_image_layout(self.layout, is_ds) */ unimplemented!(),
        );

        let info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::ImageViewCreateInfo,
            p_next: ::null(),
            flags,
            image: image.vk_image(),
            view_type,
            format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::Identity,
                g: vk::ComponentSwizzle::Identity,
                b: vk::ComponentSwizzle::Identity,
                a: vk::ComponentSwizzle::Identity,
            },
            subresource_range: translate_image_subresource_range(
                &self.subrange,
                image.meta().image_aspects(),
            ),
        };

        let vk_device = self.device.vk_device();
        let vk_image_view = unsafe { vk_device.create_image_view(&info, None) }
            .map_err(translate_generic_error_unwrap)?;

        unimplemented!()
        /* Ok(ImageView {
            vk_image_view,
            meta,
        }.into()) */
    }
}

/// Implementation of `ImageView` for Vulkan.
#[derive(Debug, Clone)]
pub struct ImageView {
    vk_image_view: vk::ImageView,
    meta: ImageViewMeta,
}

// zangfx_impl_handle! { ImageView, base::ImageView }

unsafe impl Sync for ImageView {}
unsafe impl Send for ImageView {}

impl ImageView {
    pub unsafe fn from_raw(vk_image_view: vk::ImageView, meta: ImageViewMeta) -> Self {
        Self {
            vk_image_view,
            meta,
        }
    }

    pub fn vk_image_view(&self) -> vk::ImageView {
        self.vk_image_view
    }

    pub(super) fn meta(&self) -> ImageViewMeta {
        self.meta
    }

    pub(super) unsafe fn destroy(&self, vk_device: &crate::AshDevice) {
        vk_device.destroy_image_view(self.vk_image_view, None);
    }
}

/// Compact representation of image view metadata. Stored in `ImageView` along
/// with the vulkan image view handle.
///
/// # Bit Fields
///
///  - `[0]`: The image layout
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageViewMeta(u8);

impl ImageViewMeta {
    pub fn new(image_layout: vk::ImageLayout) -> Self {
        let mut bits = 0;
        match image_layout {
            vk::ImageLayout::General => {
                bits.set_bit(0);
            }
            vk::ImageLayout::ShaderReadOnlyOptimal => {}
            _ => panic!("bad image layout"),
        }
        ImageViewMeta(bits)
    }

    pub fn image_layout(&self) -> vk::ImageLayout {
        if self.0.get_bit(0) {
            vk::ImageLayout::General
        } else {
            vk::ImageLayout::ShaderReadOnlyOptimal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_meta() {
        let meta = ImageMeta::new(
            vk::ImageViewType::Type2dArray,
            vk::IMAGE_ASPECT_DEPTH_BIT | vk::IMAGE_ASPECT_METADATA_BIT,
            vk::Format::R8Sscaled,
        );
        assert_eq!(meta.image_view_type(), vk::ImageViewType::Type2dArray);
        assert_eq!(
            meta.image_aspects(),
            vk::IMAGE_ASPECT_DEPTH_BIT | vk::IMAGE_ASPECT_METADATA_BIT
        );
        assert_eq!(meta.format(), vk::Format::R8Sscaled);
    }
}
