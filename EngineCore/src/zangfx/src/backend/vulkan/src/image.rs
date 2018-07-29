//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Image` for Vulkan.
use ash::version::*;
use ash::vk;
use std::ops;
use std::sync::Arc;

use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_handle, zangfx_impl_object};

use crate::device::DeviceRef;
use crate::formats::translate_image_format;
use crate::resstate;
use crate::utils::{
    offset_range, queue_id_from_queue, translate_generic_error_unwrap,
    translate_image_subresource_range, translate_memory_req, QueueIdBuilder,
};

/// Implementation of `ImageBuilder` for Vulkan.
#[derive(Debug)]
pub struct ImageBuilder {
    device: DeviceRef,
    queue_id: QueueIdBuilder,
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
    crate fn new(device: DeviceRef) -> Self {
        Self {
            device,
            queue_id: QueueIdBuilder::new(),
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
        self.queue_id.set(queue);
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

        let device = self.device.clone();
        let vk_image = unsafe {
            let vk_device = device.vk_device();
            vk_device.create_image(&info, None)
        }.map_err(translate_generic_error_unwrap)?;

        let num_layers = match image_view_type {
            Type3d => dims[2],
            _ => array_layers,
        };

        let vulkan_image = Arc::new(VulkanImage {
            device,
            vk_image,
            num_layers,
            num_mip_levels: self.num_mip_levels,
            usage: self.usage,
            aspects: aspect,
        });

        let state = ImageState::new(&vulkan_image);

        let image_view = Arc::new(ImageView::new(
            vulkan_image,
            ImageSubRange {
                mip_levels: 0..self.num_mip_levels,
                layers: 0..num_layers,
            },
            image_view_type,
            format,
        )?);

        let queue_id = self.queue_id.get(&image_view.vulkan_image.device);

        let tracked_state = Arc::new(resstate::TrackedState::new(queue_id, state));

        Ok(Image {
            image_view,
            tracked_state,
        }.into())
    }
}

/// Implementation of `Image` for Vulkan.
#[derive(Debug, Clone)]
pub struct Image {
    image_view: Arc<ImageView>,

    /// The container for the tracked state of an image on a particular queue.
    /// Shared among all views of an image.
    tracked_state: Arc<resstate::TrackedState<ImageState>>,
}

zangfx_impl_handle! { Image, base::ImageRef }

/// An image view representing a subresource of an image.
#[derive(Debug)]
crate struct ImageView {
    vulkan_image: Arc<VulkanImage>,
    vk_image_view: vk::ImageView,
    format: vk::Format,
    range: ImageSubRange,
    view_type: vk::ImageViewType,
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            let vk_device = self.vulkan_image.device.vk_device();
            vk_device.destroy_image_view(self.vk_image_view, None);
        }
    }
}

/// The smart pointer for `vk::Image`.
#[derive(Debug)]
struct VulkanImage {
    device: DeviceRef,
    vk_image: vk::Image,
    num_layers: u32,
    num_mip_levels: u32,
    usage: base::ImageUsageFlags,
    aspects: vk::ImageAspectFlags,
    // TODO: Heap binding
}

impl Drop for VulkanImage {
    fn drop(&mut self) {
        unsafe {
            let vk_device = self.device.vk_device();
            vk_device.destroy_image(self.vk_image, None);
        }
    }
}

/// The tracked state of an image on a particular queue.
#[derive(Debug)]
crate struct ImageState {
    // TODO: Image state
}

#[derive(Debug, PartialEq, Eq, Clone)]
crate struct ImageSubRange {
    crate mip_levels: ops::Range<u32>,
    crate layers: ops::Range<u32>,
}

impl Image {
    // TODO: `Image::frm_raw`
    /*pub unsafe fn from_raw(vk_image: vk::Image, meta: ImageMeta) -> Self {
        Self { vk_image, meta }
    }*/

    pub fn vk_image(&self) -> vk::Image {
        self.image_view.vulkan_image.vk_image
    }

    pub fn vk_image_view(&self) -> vk::ImageView {
        self.image_view.vk_image_view
    }

    crate fn aspects(&self) -> vk::ImageAspectFlags {
        self.image_view.vulkan_image.aspects
    }

    crate fn translate_layout(&self, value: base::ImageLayout) -> vk::ImageLayout {
        self.image_view.vulkan_image.translate_layout(value)
    }

    crate fn resolve_vk_subresource_layers(
        &self,
        value: &base::ImageLayerRange,
        aspect_mask: vk::ImageAspectFlags,
    ) -> vk::ImageSubresourceLayers {
        self.image_view
            .resolve_vk_subresource_layers(value, aspect_mask)
    }
}

impl ImageState {
    fn new(_vulkan_image: &Arc<VulkanImage>) -> Self {
        Self {}
    }
}

impl ImageView {
    fn new(
        vulkan_image: Arc<VulkanImage>,
        subrange: ImageSubRange,
        view_type: vk::ImageViewType,
        format: vk::Format,
    ) -> Result<Self> {
        let flags = vk::ImageViewCreateFlags::empty();
        // flags: "reserved for future use"

        let info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::ImageViewCreateInfo,
            p_next: ::null(),
            flags,
            image: vulkan_image.vk_image,
            view_type,
            format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::Identity,
                g: vk::ComponentSwizzle::Identity,
                b: vk::ComponentSwizzle::Identity,
                a: vk::ComponentSwizzle::Identity,
            },
            subresource_range: translate_image_subresource_range(
                &(subrange.clone()).into(),
                vulkan_image.aspects,
            ),
        };

        let vk_image_view = unsafe {
            let vk_device = vulkan_image.device.vk_device();
            vk_device.create_image_view(&info, None)
        }.map_err(translate_generic_error_unwrap)?;

        Ok(Self {
            vulkan_image,
            vk_image_view,
            format,
            range: subrange,
            view_type,
        })
    }

    fn resolve_subrange(&self, range: &base::ImageSubRange) -> ImageSubRange {
        let ref base_mip_levels = self.range.mip_levels;
        let ref base_layers = self.range.layers;
        ImageSubRange {
            mip_levels: offset_range(
                range
                    .mip_levels
                    .clone()
                    .unwrap_or_else(|| 0..(base_mip_levels.end - base_mip_levels.start)),
                base_mip_levels.start,
            ),
            layers: offset_range(
                range
                    .layers
                    .clone()
                    .unwrap_or_else(|| 0..(base_layers.end - base_layers.start)),
                base_layers.start,
            ),
        }
    }

    fn resolve_vk_subresource_layers(
        &self,
        value: &base::ImageLayerRange,
        aspect_mask: vk::ImageAspectFlags,
    ) -> vk::ImageSubresourceLayers {
        let ref layers = value.layers;

        let ref base_mip_levels = self.range.mip_levels;
        let ref base_layers = self.range.layers;

        vk::ImageSubresourceLayers {
            aspect_mask,
            mip_level: value.mip_level + base_mip_levels.start,
            base_array_layer: layers.start + base_layers.start,
            layer_count: layers.end - layers.start,
        }
    }
}

impl VulkanImage {
    crate fn translate_layout(&self, value: base::ImageLayout) -> vk::ImageLayout {
        translate_image_layout(
            self.usage,
            value,
            self.aspects
                .intersects(vk::IMAGE_ASPECT_DEPTH_BIT | vk::IMAGE_ASPECT_STENCIL_BIT),
        )
    }
}

impl base::Image for Image {
    fn build_image_view(&self) -> base::ImageViewBuilderRef {
        Box::new(ImageViewBuilder::new(self.clone()))
    }

    fn make_proxy(&self, queue: &base::CmdQueueRef) -> base::ImageRef {
        let queue_id = queue_id_from_queue(queue);

        let image_view = self.image_view.clone();

        // Create a fresh tracked state for the target queue
        let state = ImageState::new(&self.image_view.vulkan_image);
        let tracked_state = Arc::new(resstate::TrackedState::new(queue_id, state));

        Image {
            image_view,
            tracked_state,
        }.into()
    }

    fn get_memory_req(&self) -> Result<base::MemoryReq> {
        let vk_device = self.image_view.vulkan_image.device.vk_device();
        Ok(translate_memory_req(
            &vk_device.get_image_memory_requirements(self.vk_image()),
        ))
    }
}

impl Into<base::ImageSubRange> for ImageSubRange {
    fn into(self) -> base::ImageSubRange {
        base::ImageSubRange {
            mip_levels: Some(self.mip_levels.clone()),
            layers: Some(self.layers.clone()),
        }
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

crate fn translate_image_layout(
    usage: base::ImageUsageFlags,
    value: base::ImageLayout,
    is_depth_stencil: bool,
) -> vk::ImageLayout {
    let mutable = usage.contains(base::ImageUsage::Mutable);
    let storage = usage.contains(base::ImageUsage::Storage);

    match (value, is_depth_stencil, mutable, storage) {
        // The `Mutable` flag takes precedence over anything - It forces the use
        // of the generic image layout
        (_, _, true, _) => vk::ImageLayout::General,

        // Layouts for the fixed-function pipeline
        (base::ImageLayout::Render, false, false, _) => vk::ImageLayout::ColorAttachmentOptimal,
        (base::ImageLayout::Render, true, false, _) => {
            vk::ImageLayout::DepthStencilAttachmentOptimal
        }
        (base::ImageLayout::CopyRead, _, false, _) => vk::ImageLayout::TransferSrcOptimal,
        (base::ImageLayout::CopyWrite, _, false, _) => vk::ImageLayout::TransferDstOptimal,

        // Can use the `SHADER_READ_ONLY` if the image is never used as a
        // storage image
        (base::ImageLayout::Shader, _, false, false) => vk::ImageLayout::ShaderReadOnlyOptimal,
        (base::ImageLayout::Shader, _, false, true) => vk::ImageLayout::General,
    }
}

/// Implementation of `ImageViewBuilder` for Vulkan.
#[derive(Debug)]
pub struct ImageViewBuilder {
    image: Image,
    subrange: base::ImageSubRange,
    format: Option<base::ImageFormat>,
    image_type: Option<base::ImageType>,
}

zangfx_impl_object! { ImageViewBuilder: dyn base::ImageViewBuilder, dyn (crate::Debug) }

impl ImageViewBuilder {
    fn new(image: Image) -> Self {
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
        let ref image: Image = self.image;

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
            .unwrap_or(image.image_view.view_type);

        let format = self
            .format
            .map(|f| translate_image_format(f).expect("unsupported image format"))
            .unwrap_or(image.image_view.format);

        let image_view = Arc::new(ImageView::new(
            image.image_view.vulkan_image.clone(),
            image.image_view.resolve_subrange(&self.subrange),
            view_type,
            format,
        )?);

        let tracked_state = image.tracked_state.clone();

        Ok(Image {
            image_view,
            tracked_state,
        }.into())
    }
}
