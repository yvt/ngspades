//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use backend_vulkan;
use core;
use wsi_core;
use std::sync::{Arc, Mutex};
use std::{fmt, ptr};
use ash::vk;
use ash::extensions::Swapchain as AshSwapchain;

use cgmath::Vector3;

use backend_vulkan::ManagedDeviceRef;
use backend_vulkan::imp::translate_generic_error_unwrap;
use {ManagedDevice, ManagedImage};
use colorspace::reverse_translate_color_space;

type ManagedFence = backend_vulkan::imp::Fence<ManagedDeviceRef>;

#[derive(Debug)]
pub struct Drawable {
    data: Arc<SwapchainData>,
    image_index: u32,
    acq_fence: ManagedFence,
    rel_fence: ManagedFence,
}

impl wsi_core::Drawable for Drawable {
    type Backend = backend_vulkan::ManagedBackend;

    fn image(&self) -> &<Self::Backend as core::Backend>::Image {
        &self.data.images[self.image_index as usize]
    }
    fn acquiring_fence(&self) -> Option<&<Self::Backend as core::Backend>::Fence> {
        Some(&self.acq_fence)
    }
    fn releasing_fence(&self) -> Option<&<Self::Backend as core::Backend>::Fence> {
        Some(&self.rel_fence)
    }
    fn finalize(
        &self,
        command_buffer: &mut <Self::Backend as core::Backend>::CommandBuffer,
        _stage: core::PipelineStageFlags,
        _access: core::AccessTypeFlags,
        _layout: core::ImageLayout,
    ) {
        // TODO: layout transition and queue family ownership transtiion
        let ref _cfg: &SwapchainConfig = self.data.cfg.as_ref().unwrap();
        let _vk_cb = command_buffer.active_command_buffer().unwrap();
    }
    fn present(&self) {
        use core::Device;
        let ref cfg: &SwapchainConfig = self.data.cfg.as_ref().unwrap();

        let queue = cfg.device.main_queue();
        let mut queue_lock = queue.lock();
        let mut fence_lock = self.rel_fence.lock(&mut queue_lock);

        let (sem, signaled) = fence_lock.get_external_semaphore(0);

        let ret = unsafe {
            cfg.swapchain_loader.queue_present_khr(
                cfg.present_queue,
                &vk::PresentInfoKHR {
                    s_type: vk::StructureType::PresentInfoKhr,
                    p_next: ptr::null(),
                    wait_semaphore_count: if signaled {
                        1
                    } else {
                        // whatever this means
                        0
                    },
                    p_wait_semaphores: [sem].as_ptr(),
                    swapchain_count: 1,
                    p_swapchains: [cfg.swapchain].as_ptr(),
                    p_image_indices: [self.image_index].as_ptr(),
                    p_results: ptr::null_mut(),
                },
            )
        };

        if ret.is_ok() {
            unsafe {
                fence_lock.unsignal_external_semaphore(0);
            }
        }

        // TODO: handle error returned by `queue_present_khr`
    }
}

/// Wraps `VK_KHR_swapchain`'s swapchain.
///
/// Destroys the swapchain automatically when dropped.
#[derive(Debug)]
pub struct Swapchain {
    data: Arc<SwapchainData>,
}

#[derive(Debug)]
struct SwapchainData {
    cfg: Option<SwapchainConfig>,
    images: Vec<ManagedImage>,
    fences: Mutex<[Option<(ManagedFence, ManagedFence)>; backend_vulkan::imp::MAX_NUM_QUEUES]>,
}

#[derive(Clone)]
pub struct SwapchainConfig {
    pub device: Arc<ManagedDevice>,
    pub swapchain_loader: AshSwapchain,
    pub swapchain: vk::SwapchainKHR,
    pub present_queue: vk::Queue,
    pub present_queue_family: u32,
    pub info: wsi_core::DrawableInfo,
}

impl fmt::Debug for SwapchainConfig {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("SwapchainConfig")
            .field("device", &self.device)
            .field("swapchain", &self.swapchain)
            .field("present_queue", &self.present_queue)
            .field("info", &self.info)
            .finish()
    }
}

impl Swapchain {
    pub unsafe fn from_raw(cfg: SwapchainConfig) -> core::Result<Self> {
        let images = cfg.swapchain_loader
            .get_swapchain_images_khr(cfg.swapchain)
            .map_err(translate_generic_error_unwrap)?
            .iter()
            .map(|vk_image| ManagedImage::import(*vk_image))
            .collect();
        Ok(Self {
            data: Arc::new(SwapchainData {
                cfg: Some(cfg),
                fences: Mutex::new(Default::default()),
                images,
            }),
        })
    }

    pub fn config(&self) -> &SwapchainConfig {
        self.data.cfg.as_ref().unwrap()
    }

    fn ensure_image_unique_ownership(&mut self) -> bool {
        if let Some(data) = Arc::get_mut(&mut self.data) {
            data.ensure_image_unique_ownership()
        } else {
            false
        }

    }

    pub fn try_take(mut self) -> Result<SwapchainConfig, Self> {
        if !self.ensure_image_unique_ownership() {
            return Err(self);
        }
        match Arc::try_unwrap(self.data) {
            Ok(mut data) => Ok(data.cfg.take().unwrap()),
            Err(data) => Err(Self { data }),
        }
    }
}

impl SwapchainData {
    fn ensure_image_unique_ownership(&mut self) -> bool {
        let mut new_images = Vec::new();
        let mut fail = false;
        for image in self.images.drain(..) {
            match image.try_take() {
                Ok(vk_image) => new_images.push(unsafe { ManagedImage::import(vk_image) }),
                Err(image) => {
                    fail = true;
                    new_images.push(image)
                }
            }
        }
        self.images = new_images;
        !fail
    }
}

impl Drop for SwapchainData {
    fn drop(&mut self) {
        assert!(
            self.ensure_image_unique_ownership(),
            "One of swapchain images is still in use"
        );
        if let Some(ref cfg) = self.cfg {
            unsafe {
                cfg.swapchain_loader.destroy_swapchain_khr(
                    cfg.swapchain,
                    None,
                );
            }
        }
    }
}

impl wsi_core::Swapchain for Swapchain {
    type Backend = backend_vulkan::ManagedBackend;
    type Drawable = Drawable;

    fn device(&self) -> &<Self::Backend as core::Backend>::Device {
        &self.data.cfg.as_ref().unwrap().device
    }

    fn next_drawable(
        &self,
        description: &wsi_core::FrameDescription,
    ) -> Result<Self::Drawable, wsi_core::SwapchainError> {
        use ngsgfx_common::int::BinaryInteger;
        use core::Device;

        let config: &backend_vulkan::imp::DeviceConfig = self.device().config();
        let iqs = config.engine_queue_mappings.internal_queues_for_engines(
            description.acquiring_engines,
        );

        assert!(
            iqs.is_power_of_two(),
            "unsuppported: acquiring fence cannot be waited by multiple or zero internal queues {}"
        );


        let iq = iqs.one_digits().nth(0).unwrap();

        let (acq_fence, rel_fence) = {
            let mut fences = self.data.fences.lock().unwrap();
            if fences[iq as usize].is_none() {
                let queue = self.device().main_queue();
                fences[iq as usize] = Some((
                    // acquiring fence
                    ManagedFence::new(queue, iqs, 0).map_err(
                        wsi_core::SwapchainError::GenericError,
                    )?,

                    // releasing fence
                    ManagedFence::new(queue, 0u32, 1).map_err(
                        wsi_core::SwapchainError::GenericError,
                    )?,
                ));
            }
            fences[iq as usize].clone().unwrap()
        };

        let image_index = {
            let queue = self.device().main_queue();
            let mut queue_lock = queue.lock();
            let mut fence_lock = acq_fence.lock(&mut queue_lock);

            let (acq_fence_sem, signaled) = fence_lock
                .get_internal_queue_semaphore(iq as usize)
                .unwrap();

            // assumes acquiring semaphores are *always* unsignaled between any two
            // consecutive calls to `next_drawable`
            // TODO: remove this assumption
            assert!(!signaled, "acquiring semaphore is not unsignaled");

            let ref cfg: &SwapchainConfig = self.data.cfg.as_ref().unwrap();
            let image_index = unsafe {
                cfg.swapchain_loader.acquire_next_image_khr(
                    cfg.swapchain,
                    1000000000,
                    acq_fence_sem,
                    vk::Fence::null(),
                )
            }.map_err(translate_acquire_next_image_error_unwrap)?;

            unsafe {
                fence_lock.signal_internal_queue_semaphore(iq as usize);
            }
            image_index
        };

        Ok(Drawable {
            data: self.data.clone(),
            image_index,
            acq_fence,
            rel_fence,
        })
    }

    fn drawable_info(&self) -> wsi_core::DrawableInfo {
        self.data.cfg.as_ref().unwrap().info.clone()
    }
}

pub(crate) fn translate_acquire_next_image_error(
    result: vk::Result,
) -> Result<wsi_core::SwapchainError, vk::Result> {
    use wsi_core::SwapchainError;
    match result {
        vk::Result::Timeout => Ok(SwapchainError::NotReady),
        vk::Result::NotReady => Ok(SwapchainError::NotReady),
        // TODO: Actually `SuboptimalKhr` is a success code but `ash` handles it in a wrong way.
        vk::Result::SuboptimalKhr => Ok(SwapchainError::OutOfDate),
        vk::Result::ErrorOutOfDateKhr => Ok(SwapchainError::OutOfDate),
        vk::Result::ErrorSurfaceLostKhr => Ok(SwapchainError::TargetLost),
        result => {
            backend_vulkan::imp::translate_generic_error(result).map(SwapchainError::GenericError)
        }
    }
}

pub(crate) fn translate_acquire_next_image_error_unwrap(
    result: vk::Result,
) -> wsi_core::SwapchainError {
    translate_acquire_next_image_error(result).unwrap()
}

pub fn drawable_info_from_swapchain_info(
    info: &vk::SwapchainCreateInfoKHR,
) -> wsi_core::DrawableInfo {
    wsi_core::DrawableInfo {
        extents: Vector3::new(info.image_extent.width, info.image_extent.height, 1),
        num_array_layers: info.image_array_layers,
        format: backend_vulkan::imp::reverse_translate_image_format(info.image_format)
            .expect("unsupported image format"),
        colorspace: reverse_translate_color_space(info.image_color_space)
            .expect("unsupported color space"),
    }
}
