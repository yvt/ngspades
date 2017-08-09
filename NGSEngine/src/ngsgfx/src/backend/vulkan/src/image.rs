//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, RefEqArc, DeviceRef, AshDevice, translate_generic_error_unwrap, translate_image_layout,
     translate_image_subresource_range};
use imp::{MemoryHunk, translate_image_format, LlFence};
use command::mutex::{ResourceMutex, ResourceMutexDeviceRef};

use ash::vk;
use ash::version::DeviceV1_0;
use std::sync::Arc;
use std::{mem, ptr};

pub fn translate_image_usage(value: core::ImageUsageFlags) -> vk::ImageUsageFlags {
    let mut usage = vk::ImageUsageFlags::empty();
    if value.contains(core::ImageUsage::TransferSource) {
        usage |= vk::IMAGE_USAGE_TRANSFER_SRC_BIT;
    }
    if value.contains(core::ImageUsage::TransferDestination) {
        usage |= vk::IMAGE_USAGE_TRANSFER_DST_BIT;
    }
    if value.contains(core::ImageUsage::Sampled) {
        usage |= vk::IMAGE_USAGE_SAMPLED_BIT;
    }
    if value.contains(core::ImageUsage::Storage) {
        usage |= vk::IMAGE_USAGE_STORAGE_BIT;
    }
    if value.contains(core::ImageUsage::ColorAttachment) {
        usage |= vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT;
    }
    if value.contains(core::ImageUsage::DepthStencilAttachment) {
        usage |= vk::IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT;
    }
    if value.contains(core::ImageUsage::TransientAttachment) {
        usage |= vk::IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT;
    }
    if value.contains(core::ImageUsage::InputAttachment) {
        usage |= vk::IMAGE_USAGE_INPUT_ATTACHMENT_BIT;
    }
    usage
}

pub(crate) struct UnassociatedImage<'a, T: DeviceRef> {
    device_ref: &'a T,
    handle: vk::Image,
    info: ImageInfo,
}

impl<'a, T: DeviceRef> UnassociatedImage<'a, T> {
    pub(crate) fn new(device_ref: &'a T, desc: &core::ImageDescription) -> core::Result<Self> {
        let mut flags = vk::ImageCreateFlags::empty();
        if desc.flags.contains(core::ImageFlag::MutableFormat) {
            flags |= vk::IMAGE_CREATE_MUTABLE_FORMAT_BIT;
        }
        if desc.image_type == core::ImageType::Cube ||
            desc.image_type == core::ImageType::CubeArray
        {
            // note: NgsGFX does not allow creating cube image views from
            // other kinds of images
            flags |= vk::IMAGE_CREATE_CUBE_COMPATIBLE_BIT;
        }

        let image_type = match desc.image_type {
            core::ImageType::OneD => vk::ImageType::Type1d,
            core::ImageType::TwoD |
            core::ImageType::TwoDArray |
            core::ImageType::Cube |
            core::ImageType::CubeArray => vk::ImageType::Type2d,
            core::ImageType::ThreeD => vk::ImageType::Type3d,
        };

        let tiling = match desc.tiling {
            core::ImageTiling::Linear => vk::ImageTiling::Linear,
            core::ImageTiling::Optimal => vk::ImageTiling::Optimal,
        };

        let usage = translate_image_usage(desc.usage);

        let info = vk::ImageCreateInfo {
            s_type: vk::StructureType::ImageCreateInfo,
            p_next: ptr::null(),
            flags,
            image_type,
            format: translate_image_format(desc.format).expect("unsupported image format"),
            extent: vk::Extent3D {
                width: desc.extent.x,
                height: desc.extent.y,
                depth: desc.extent.z,
            },
            mip_levels: desc.num_mip_levels,
            array_layers: desc.num_array_layers,
            samples: vk::SAMPLE_COUNT_1_BIT,
            tiling,
            usage,
            sharing_mode: vk::SharingMode::Exclusive,
            queue_family_index_count: 0, // ignored for `SharingMode::Exclusive`
            p_queue_family_indices: ptr::null(),
            initial_layout: translate_image_layout(desc.initial_layout),
        };

        let mut aspect = vk::ImageAspectFlags::empty();
        if desc.format.has_color() {
            aspect |= vk::IMAGE_ASPECT_COLOR_BIT;
        }
        if desc.format.has_depth() {
            aspect |= vk::IMAGE_ASPECT_DEPTH_BIT;
        }
        if desc.format.has_stencil() {
            aspect |= vk::IMAGE_ASPECT_STENCIL_BIT;
        }
        let img_info = ImageInfo { aspect };

        let device: &AshDevice = device_ref.device();
        let handle = unsafe { device.create_image(&info, device_ref.allocation_callbacks()) }
            .map_err(translate_generic_error_unwrap)?;

        Ok(UnassociatedImage {
            device_ref,
            handle,
            info: img_info,
        })
    }

    pub(crate) fn memory_requirements(&self) -> vk::MemoryRequirements {
        let device: &AshDevice = self.device_ref.device();
        device.get_image_memory_requirements(self.handle)
    }

    pub(crate) fn into_raw(mut self) -> vk::Image {
        mem::replace(&mut self.handle, vk::Image::null())
    }

    pub(crate) fn associate(
        self,
        hunk: Arc<MemoryHunk<T>>,
        offset: vk::DeviceSize,
    ) -> core::Result<Image<T>> {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.bind_image_memory(self.handle, hunk.handle(), offset) }
            .map_err(translate_generic_error_unwrap)?;

        Ok(Image {
            data: RefEqArc::new(ImageData {
                mutex: ResourceMutex::new(
                    ImageLockData {
                        hunk: Some(hunk),
                        info: self.info.clone(),
                        handle: self.into_raw(),
                    },
                    false,
                ),
            }),
        })
    }
}

impl<'a, T: DeviceRef> Drop for UnassociatedImage<'a, T> {
    fn drop(&mut self) {
        if self.handle != vk::Image::null() {
            let device: &AshDevice = self.device_ref.device();
            unsafe { device.destroy_image(self.handle, self.device_ref.allocation_callbacks()) };
        }
    }
}

pub struct Image<T: DeviceRef> {
    data: RefEqArc<ImageData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Image<T> => data
}

#[derive(Debug)]
struct ImageData<T: DeviceRef> {
    mutex: ResourceMutex<LlFence<T>, ImageLockData<T>>,
}

#[derive(Debug)]
pub(crate) struct ImageLockData<T: DeviceRef> {
    hunk: Option<Arc<MemoryHunk<T>>>,
    handle: vk::Image,
    info: ImageInfo,
}

#[derive(Debug, Clone)]
pub(crate) struct ImageInfo {
    pub aspect: vk::ImageAspectFlags,
}

impl<T: DeviceRef> core::Image for Image<T> {}

impl<T: DeviceRef> core::Marker for Image<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> Drop for ImageLockData<T> {
    fn drop(&mut self) {
        if let Some(ref hunk) = self.hunk {
            let device_ref = hunk.device_ref();
            let device: &AshDevice = device_ref.device();
            unsafe { device.destroy_image(self.handle, device_ref.allocation_callbacks()) };
        }
    }
}

impl<T: DeviceRef> Image<T> {
    /// Construct `Image` from an externally provided `vk::Image`.
    ///
    /// The given `image` will *not* be destroyed automatically.
    ///
    /// It is the caller's responsibility to make sure the `Image` is not used after
    /// `image` was destroyed. For example, `try_take` can be used to take back the
    /// ownership of the `vk::Image`.
    ///
    /// The image must be a color image.
    pub unsafe fn import(image: vk::Image) -> Self {
        assert!(image != vk::Image::null());

        // Make `ResourceMutex` mutable so it can be `try_take`-ed later
        Self {
            data: RefEqArc::new(ImageData {
                mutex: ResourceMutex::new(
                    ImageLockData {
                        hunk: None,
                        handle: image,
                        info: ImageInfo { aspect: vk::IMAGE_ASPECT_COLOR_BIT },
                    },
                    true,
                ),
            }),
        }
    }

    /// Take the contained `vk::Image` if there are no other references
    /// (including device access) to it.
    ///
    /// The `Image` must have been created with `import`. Otherwise a panic will occur.
    pub fn try_take(self) -> Result<vk::Image, Self> {
        if self.data.mutex.get_host_read().hunk.is_some() {
            // Can't take a managed image.
            panic!("cannot take a managed image");
        }
        match RefEqArc::try_unwrap(self.data) {
            Ok(mut data) => {
                if let Some(lock_data) = data.mutex.try_lock_host_write(false) {
                    // The image is not in use - we can take it
                    return Ok(lock_data.handle);
                }

                // The device is still accessing the `vk::Image`
                Err(Self { data: RefEqArc::new(data) })
            }
            // There are other references to `ImageData`
            Err(data) => Err(Self { data }),
        }
    }

    pub fn handle(&self) -> vk::Image {
        self.data.mutex.get_host_read().handle
    }

    pub(crate) fn info(&self) -> &ImageInfo {
        &self.data.mutex.get_host_read().info
    }

    pub(crate) fn lock_device(&self) -> ResourceMutexDeviceRef<LlFence<T>, ImageLockData<T>> {
        self.data.mutex.expect_device_access().0
    }
}

pub struct ImageView<T: DeviceRef> {
    data: RefEqArc<ImageViewData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for ImageView<T> => data
}

#[derive(Debug)]
struct ImageViewData<T: DeviceRef> {
    device_ref: T,
    image_data: RefEqArc<ImageData<T>>,
    handle: vk::ImageView,
}

impl<T: DeviceRef> core::ImageView for ImageView<T> {}

impl<T: DeviceRef> core::Marker for ImageView<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> Drop for ImageViewData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.destroy_image_view(self.handle, self.device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> ImageView<T> {
    pub(crate) fn new(
        device_ref: &T,
        desc: &core::ImageViewDescription<Image<T>>,
        _: &core::DeviceCapabilities,
    ) -> core::Result<Self> {
        // TODO: validate compatibility with the image

        let flags = vk::ImageViewCreateFlags::empty();
        // flags: "reserved for future use"

        let view_type = match desc.image_type {
            core::ImageType::OneD => vk::ImageViewType::Type1d,
            core::ImageType::TwoD => vk::ImageViewType::Type2d,
            core::ImageType::TwoDArray => vk::ImageViewType::Type2dArray,
            core::ImageType::ThreeD => vk::ImageViewType::Type3d,
            core::ImageType::Cube => vk::ImageViewType::Cube,
            core::ImageType::CubeArray => vk::ImageViewType::CubeArray,
        };

        let mut aspect_mask = vk::ImageAspectFlags::empty();

        if desc.format.has_color() {
            aspect_mask |= vk::IMAGE_ASPECT_COLOR_BIT;
        }
        if desc.format.has_depth() {
            aspect_mask |= vk::IMAGE_ASPECT_DEPTH_BIT;
        }
        if desc.format.has_stencil() {
            aspect_mask |= vk::IMAGE_ASPECT_STENCIL_BIT;
        }

        let info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::ImageViewCreateInfo,
            p_next: ptr::null(),
            flags,
            image: desc.image.handle(),
            view_type,
            format: translate_image_format(desc.format).expect("unsupported image format"),
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::Identity,
                g: vk::ComponentSwizzle::Identity,
                b: vk::ComponentSwizzle::Identity,
                a: vk::ComponentSwizzle::Identity,
            },
            subresource_range: translate_image_subresource_range(&desc.range, aspect_mask),
        };

        let device: &AshDevice = device_ref.device();
        let handle = unsafe { device.create_image_view(&info, device_ref.allocation_callbacks()) }
            .map_err(translate_generic_error_unwrap)?;

        Ok(ImageView {
            data: RefEqArc::new(ImageViewData {
                device_ref: device_ref.clone(),
                image_data: desc.image.data.clone(),
                handle,
            }),
        })
    }

    pub fn handle(&self) -> vk::ImageView {
        self.data.handle
    }
}
