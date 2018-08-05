//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Image` for Vulkan.
use ash::version::*;
use ash::{prelude::VkResult, vk};
use ngsenumflags::flags;
use smallvec::{smallvec, SmallVec};
use std::ops;
use std::sync::Arc;

use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_handle, zangfx_impl_object};
use zangfx_common::{FreezableCell, FreezableCellRef};

use crate::device::DeviceRef;
use crate::formats::translate_image_format;
use crate::utils::{
    offset_range, queue_id_from_queue, translate_generic_error_unwrap,
    translate_image_subresource_range, translate_memory_req, QueueIdBuilder,
};
use crate::{heap, resstate};

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

        let vulkan_image = Arc::new(VulkanImage {
            device,
            vk_image,
            num_layers: array_layers,
            num_mip_levels: self.num_mip_levels,
            usage: self.usage,
            aspects: aspect,
            binding_info: heap::HeapBindingInfo::new(),
            destroy_manually: false,
        });

        let state = ImageState::new(&vulkan_image, true);

        let queue_id = self.queue_id.get(&vulkan_image.device);

        let image_view = Arc::new(ImageView::new_prototype(
            vulkan_image,
            ImageSubRange {
                mip_levels: 0..self.num_mip_levels,
                layers: 0..array_layers,
            },
            image_view_type,
            format,
            queue_id,
        ));

        let tracked_state = Arc::new(resstate::TrackedState::new(queue_id, state));

        Ok(Image {
            image_view,
            tracked_state,
        }.into())
    }
}

/// Used to import a raw Vulkan image handle.
#[derive(Debug, Clone, Copy)]
pub struct ImportImage {
    pub vk_image: vk::Image,
    pub format: vk::Format,
    pub view_type: vk::ImageViewType,
    pub num_mip_levels: u32,
    pub num_layers: u32,
    pub usage: base::ImageUsageFlags,
    pub aspects: vk::ImageAspectFlags,
    pub destroy_manually: bool,
}

impl ImportImage {
    pub unsafe fn build(&self, queue: &crate::cmd::queue::CmdQueue) -> Result<Image> {
        let device = queue.device().clone();

        let vulkan_image = Arc::new(VulkanImage {
            device,
            vk_image: self.vk_image,
            num_layers: self.num_layers,
            num_mip_levels: self.num_mip_levels,
            usage: self.usage,
            aspects: self.aspects,
            binding_info: heap::HeapBindingInfo::new(),
            destroy_manually: self.destroy_manually,
        });

        let state = ImageState::new(&vulkan_image, true);

        let queue_id = queue.resstate_queue_id();

        let image_view = Arc::new(ImageView::new(
            vulkan_image,
            ImageSubRange {
                mip_levels: 0..self.num_mip_levels,
                layers: 0..self.num_layers,
            },
            self.view_type,
            self.format,
            queue_id,
        )?);

        let tracked_state = Arc::new(resstate::TrackedState::new(queue_id, state));

        Ok(Image {
            image_view,
            tracked_state,
        })
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
    /// The Vulkan image view handle. If this `ImageView` is created as the
    /// primary image view of an image (i.e., not created by `build_image_view`),
    /// this can be in the "unfrozen" state, containing a null handle.
    /// This is because `VkImageView` cannot be created until `VkImage` is bound
    /// to a `VkDeviceMemory`.
    ///
    /// Otherwise, this is guaranteed to be in the "frozen" state, containing a
    /// non-null handle.
    vk_image_view: FreezableCell<vk::ImageView>,
    format: vk::Format,
    range: ImageSubRange,
    view_type: vk::ImageViewType,

    /// Used for automatic lifetime tracking.
    tracked_state: resstate::TrackedState<ImageViewState>,
}

crate type ImageViewState = ();

impl Drop for ImageView {
    fn drop(&mut self) {
        let vk_image_view = *self.vk_image_view.get_mut();
        if vk_image_view != vk::ImageView::null() {
            unsafe {
                let vk_device = self.vulkan_image.device.vk_device();
                vk_device.destroy_image_view(vk_image_view, None);
            }
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
    binding_info: heap::HeapBindingInfo,
    destroy_manually: bool,
}

impl Drop for VulkanImage {
    fn drop(&mut self) {
        if !self.destroy_manually {
            unsafe {
                let vk_device = self.device.vk_device();
                vk_device.destroy_image(self.vk_image, None);
            }
        }
    }
}

/// The tracked state of an image on a particular queue.
#[derive(Debug)]
crate struct ImageState {
    /// `ImageUnitState` for each state-tracking unit. The subresource each
    /// element represents varies depending on the image usage flags.
    /// The mapping can be computed by using `ImageStateAddresser`
    crate units: SmallVec<[ImageUnitState; 1]>,
}

/// The tracked state of an image layer on a particular queue.
#[derive(Debug, Clone)]
crate struct ImageUnitState {
    /// The current image layout.
    crate layout: Option<vk::ImageLayout>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
crate struct ImageSubRange {
    crate mip_levels: ops::Range<u32>,
    crate layers: ops::Range<u32>,
}

impl Image {
    pub fn vk_image(&self) -> vk::Image {
        self.image_view.vulkan_image.vk_image
    }

    pub fn vk_image_view(&self) -> vk::ImageView {
        *(self.image_view.vk_image_view)
            .frozen_borrow()
            .expect("image is not bound to a heap")
    }

    crate fn aspects(&self) -> vk::ImageAspectFlags {
        self.image_view.vulkan_image.aspects
    }

    pub fn translate_layout(&self, value: base::ImageLayout) -> vk::ImageLayout {
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

    fn resolve_layer_range(&self, value: &base::ImageLayerRange) -> base::ImageLayerRange {
        self.image_view.resolve_layer_range(value)
    }

    crate fn image_view(&self) -> &Arc<ImageView> {
        &self.image_view
    }
}

impl resstate::Resource for Image {
    type State = ImageState;

    fn tracked_state(&self) -> &resstate::TrackedState<Self::State> {
        &self.tracked_state
    }
}

impl resstate::Resource for Arc<ImageView> {
    type State = ImageViewState;

    fn tracked_state(&self) -> &resstate::TrackedState<Self::State> {
        &self.tracked_state
    }
}

impl ImageState {
    fn new(vulkan_image: &VulkanImage, owned: bool) -> Self {
        let addresser = ImageStateAddresser::from_vulkan_image(&vulkan_image);
        let substate = ImageUnitState {
            layout: if owned {
                Some(vk::ImageLayout::Undefined)
            } else {
                None
            },
        };

        Self {
            units: smallvec![substate; addresser.len()],
        }
    }
}

impl ImageView {
    /// Construct an `ImageView` but does not create a `VkImageView`. Must be
    /// `materialize()`-ed before using it.
    fn new_prototype(
        vulkan_image: Arc<VulkanImage>,
        subrange: ImageSubRange,
        view_type: vk::ImageViewType,
        format: vk::Format,
        queue_id: resstate::QueueId,
    ) -> Self {
        Self {
            vulkan_image,
            vk_image_view: FreezableCell::new_unfrozen(vk::ImageView::null()),
            format,
            range: subrange,
            view_type,
            tracked_state: resstate::TrackedState::new(queue_id, ()),
        }
    }

    /// Construct an `ImageView` along with a `VkImageView`.
    fn new(
        vulkan_image: Arc<VulkanImage>,
        subrange: ImageSubRange,
        view_type: vk::ImageViewType,
        format: vk::Format,
        queue_id: resstate::QueueId,
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
            vk_image_view: FreezableCell::new_frozen(vk_image_view),
            format,
            range: subrange,
            view_type,
            tracked_state: resstate::TrackedState::new(queue_id, ()),
        })
    }

    fn materialize(&self) -> VkResult<()> {
        let mut vk_image_view_cell = self.vk_image_view.unfrozen_borrow_mut().unwrap();

        let flags = vk::ImageViewCreateFlags::empty();
        // flags: "reserved for future use"

        let info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::ImageViewCreateInfo,
            p_next: ::null(),
            flags,
            image: self.vulkan_image.vk_image,
            view_type: self.view_type,
            format: self.format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::Identity,
                g: vk::ComponentSwizzle::Identity,
                b: vk::ComponentSwizzle::Identity,
                a: vk::ComponentSwizzle::Identity,
            },
            subresource_range: translate_image_subresource_range(
                &(self.range.clone()).into(),
                self.vulkan_image.aspects,
            ),
        };

        let vk_image_view = unsafe {
            let vk_device = self.vulkan_image.device.vk_device();
            vk_device.create_image_view(&info, None)
        }?;

        *vk_image_view_cell = vk_image_view;
        FreezableCellRef::freeze(vk_image_view_cell);

        Ok(())
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
        crate::utils::translate_image_subresource_layers(
            &self.resolve_layer_range(value),
            aspect_mask,
        )
    }

    fn resolve_layer_range(&self, value: &base::ImageLayerRange) -> base::ImageLayerRange {
        let ref layers = value.layers;

        let ref base_mip_levels = self.range.mip_levels;
        let ref base_layers = self.range.layers;

        base::ImageLayerRange {
            mip_level: value.mip_level + base_mip_levels.start,
            layers: layers.start + base_layers.start..layers.end + base_layers.start,
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

    fn memory_req(&self) -> base::MemoryReq {
        let vk_device = self.device.vk_device();
        translate_memory_req(&vk_device.get_image_memory_requirements(self.vk_image))
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
        // FIXME: Image proxies are marked as "not owned by this queue".
        //        Is this behavior documentated somewhere?
        let state = ImageState::new(&self.image_view.vulkan_image, false);
        let tracked_state = Arc::new(resstate::TrackedState::new(queue_id, state));

        Image {
            image_view,
            tracked_state,
        }.into()
    }

    fn get_memory_req(&self) -> Result<base::MemoryReq> {
        Ok(self.image_view.vulkan_image.memory_req())
    }
}

impl heap::Bindable for Image {
    fn memory_req(&self) -> base::MemoryReq {
        self.image_view.vulkan_image.memory_req()
    }

    fn binding_info(&self) -> &heap::HeapBindingInfo {
        &self.image_view.vulkan_image.binding_info
    }

    unsafe fn bind(
        &self,
        vk_device_memory: vk::DeviceMemory,
        offset: vk::DeviceSize,
    ) -> VkResult<()> {
        let vk_device = self.image_view.vulkan_image.device.vk_device();
        vk_device.bind_image_memory(self.vk_image(), vk_device_memory, offset)?;

        // Now that the image is bound to a device memory, we can create a
        // primary image view for it.
        self.image_view.materialize()?;

        Ok(())
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

/// Color attachments always use this layout.
crate const IMAGE_LAYOUT_COLOR_ATTACHMENT: vk::ImageLayout =
    vk::ImageLayout::ColorAttachmentOptimal;

/// Depth/stencil attachments always use this layout.
crate const IMAGE_LAYOUT_DS_ATTACHMENT: vk::ImageLayout =
    vk::ImageLayout::DepthStencilAttachmentOptimal;

crate fn translate_image_layout(
    usage: base::ImageUsageFlags,
    value: base::ImageLayout,
    is_depth_stencil: bool,
) -> vk::ImageLayout {
    let mutable = usage.contains(base::ImageUsage::Mutable);
    let storage = usage.contains(base::ImageUsage::Storage);

    match (value, is_depth_stencil, mutable, storage) {
        // The render layouts cannot be controlled via the `Mutable` flag
        // because image layouts are specified as a part of the render pass
        // creation parameters (not framebuffers). We would have to re-create
        // render passes in order to change the layout.
        (base::ImageLayout::Render, false, _, _) => IMAGE_LAYOUT_COLOR_ATTACHMENT,
        (base::ImageLayout::Render, true, _, _) => IMAGE_LAYOUT_DS_ATTACHMENT,

        // The `Mutable` flag forces the use of the generic image layout
        // whenever possible
        (_, _, true, _) => vk::ImageLayout::General,

        // Layouts for the fixed-function pipeline
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
            }).unwrap_or(image.image_view.view_type);

        let format = self
            .format
            .map(|f| translate_image_format(f).expect("unsupported image format"))
            .unwrap_or(image.image_view.format);

        let image_view = Arc::new(ImageView::new(
            image.image_view.vulkan_image.clone(),
            image.image_view.resolve_subrange(&self.subrange),
            view_type,
            format,
            image.tracked_state.queue_id(),
        )?);

        let tracked_state = image.tracked_state.clone();

        Ok(Image {
            image_view,
            tracked_state,
        }.into())
    }
}

/// Maps mipmap levels and array layers to a subset of elements in an image
/// state vector.
#[derive(Debug)]
crate struct ImageStateAddresser {
    num_layers: u32,
    num_mip_levels: u32,
    num_tracked_layers: usize,
    num_tracked_mip_levels: usize,
    track_per_layer: bool,
    track_per_mip: bool,
}

impl ImageStateAddresser {
    /// Construct a `ImageStateAddresser` that can be used to compute the
    /// tracking-state unit indices for accessing the state of a given image.
    ///
    /// Since image views do not have states by themselves, this method returns
    /// an `ImageStateAddresser` of the backing image if an image view is
    /// given.
    crate fn from_image(image: &Image) -> Self {
        Self::from_vulkan_image(&image.image_view.vulkan_image)
    }

    fn from_vulkan_image(image: &VulkanImage) -> Self {
        let usage = image.usage;

        // Q. Why `Render` is included here?
        // A. If the image is marked as a render target, every layer/mip level
        // must be tracked to support the use cases where a portion of an image
        // is used as a render target and at the same time another portion is
        // access by a shader.
        let track_per_layer =
            usage.intersects(flags![base::ImageUsage::{TrackStatePerMipmapLevel | Render}]);
        let track_per_mip =
            usage.intersects(flags![base::ImageUsage::{TrackStatePerArrayLayer | Render}]);

        Self {
            num_layers: image.num_layers,
            num_mip_levels: image.num_mip_levels,
            num_tracked_layers: if track_per_layer {
                image.num_layers as usize
            } else {
                1
            },
            num_tracked_mip_levels: if track_per_mip {
                image.num_mip_levels as usize
            } else {
                1
            },
            track_per_layer,
            track_per_mip,
        }
    }

    /// Get the length of a state vector.
    crate fn len(&self) -> usize {
        self.num_tracked_layers * self.num_tracked_mip_levels
    }

    /// Return an iterator representing a subresource range of the image.
    fn indices_for_subrange(&self, range: &ImageSubRange) -> impl Iterator<Item = usize> {
        use itertools::Itertools;

        let mip_levels = if self.track_per_mip {
            range.mip_levels.clone()
        } else {
            0..1
        };
        let layers = if self.track_per_layer {
            range.layers.clone()
        } else {
            0..1
        };

        let num_tracked_mip_levels = self.num_tracked_mip_levels;

        layers
            .cartesian_product(mip_levels)
            .map(move |(layer, mip_level)| {
                mip_level as usize + layer as usize * num_tracked_mip_levels
            })
    }

    /// Return an iterator representing a subresource range of the image (view).
    crate fn indices_for_image(&self, image: &Image) -> impl Iterator<Item = usize> {
        self.indices_for_subrange(&image.image_view.range)
    }

    crate fn indices_for_image_and_layer_range(
        &self,
        image: &Image,
        range: &base::ImageLayerRange,
    ) -> impl Iterator<Item = usize> {
        let abs_range = image.resolve_layer_range(range);
        self.indices_for_subrange(&ImageSubRange {
            mip_levels: abs_range.mip_level..abs_range.mip_level + 1,
            layers: abs_range.layers,
        })
    }

    crate fn layer_range_intersects(
        &self,
        image1: &Image,
        range1: &base::ImageLayerRange,
        image2: &Image,
        range2: &base::ImageLayerRange,
    ) -> bool {
        debug_assert_eq!(image1.vk_image(), image2.vk_image());
        let abs_range1 = image1.resolve_layer_range(range1);
        let abs_range2 = image2.resolve_layer_range(range2);
        if self.track_per_mip && abs_range1.mip_level != abs_range2.mip_level {
            return false;
        }
        if self.track_per_layer {
            let ref layers1 = abs_range1.layers;
            let ref layers2 = abs_range2.layers;
            layers1.start < layers2.end && layers1.end > layers2.start
        } else {
            true
        }
    }

    crate fn subrange_for_index(&self, i: usize) -> ImageSubRange {
        if self.num_tracked_mip_levels == 1 {
            ImageSubRange {
                mip_levels: 0..self.num_mip_levels,
                layers: if self.track_per_layer {
                    i as u32..(i + 1) as u32
                } else {
                    0..self.num_layers
                },
            }
        } else if self.num_tracked_layers == 1 {
            debug_assert!(self.track_per_mip);
            ImageSubRange {
                layers: 0..self.num_layers,
                mip_levels: i as u32..(i + 1) as u32,
            }
        } else {
            debug_assert!(self.track_per_mip);
            debug_assert!(self.track_per_layer);
            let layer = (i / self.num_tracked_mip_levels) as u32;
            let mip_level = (i % self.num_tracked_mip_levels) as u32;
            ImageSubRange {
                layers: layer..layer + 1,
                mip_levels: mip_level..mip_level + 1,
            }
        }
    }
}
