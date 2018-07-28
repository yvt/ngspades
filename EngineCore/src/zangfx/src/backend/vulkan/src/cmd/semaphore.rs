//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Semaphore` for Vulkan.
//!
//! ZanGFX semaphores are functionally equivalent to Vulkan's semaphores.
//!
use ash::vk;
use ash::version::*;
use refeq::RefEqArc;

use base;
use base::Result;
use device::DeviceRef;

use utils::translate_generic_error_unwrap;

/// Implementation of `SemaphoreBuilder` for Vulkan.
#[derive(Debug)]
pub struct SemaphoreBuilder {
    device: DeviceRef,
    raw: vk::Semaphore,
}

zangfx_impl_object! { SemaphoreBuilder: base::SemaphoreBuilder, ::Debug }

impl SemaphoreBuilder {
    pub(crate) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            raw: vk::Semaphore::null(),
        }
    }

    /// Instruct the builder to use the provided `vk::Semaphore` instead of
    /// creating a new one.
    pub unsafe fn from_raw(&mut self, raw: vk::Semaphore) -> &mut Self {
        self.raw = raw;
        self
    }
}

impl base::SemaphoreBuilder for SemaphoreBuilder {
    fn build(&mut self) -> Result<base::SemaphoreRef> {
        if self.raw == vk::Semaphore::null() {
            Ok(unsafe { Semaphore::new(self.device)?.into() })
        } else {
            Ok(Semaphore {
                data: RefEqArc::new(SemaphoreData {
                    device: self.device,
                    vk_semaphore: self.raw,
                }),
            }.into())
        }
    }
}

/// Implementation of `Semaphore` for Vulkan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Semaphore {
    data: RefEqArc<SemaphoreData>,
}

zangfx_impl_handle! { Semaphore, base::SemaphoreRef }

#[derive(Debug)]
struct SemaphoreData {
    device: DeviceRef,
    vk_semaphore: vk::Semaphore,
}

impl Semaphore {
    pub(crate) unsafe fn new(device: DeviceRef) -> Result<Self> {
        let info = vk::SemaphoreCreateInfo {
            s_type: vk::StructureType::SemaphoreCreateInfo,
            p_next: ::null(),
            flags: vk::SemaphoreCreateFlags::empty(),
        };

        let vk_device: &::AshDevice = device.vk_device();

        let vk_semaphore = vk_device
            .create_semaphore(&info, None)
            .map_err(translate_generic_error_unwrap)?;

        Ok(Self {
            data: RefEqArc::new(SemaphoreData {
                device,
                vk_semaphore,
            }),
        })
    }

    pub fn vk_semaphore(&self) -> vk::Semaphore {
        self.data.vk_semaphore
    }
}

impl Drop for SemaphoreData {
    fn drop(&mut self) {
        let vk_device: &::AshDevice = self.device.vk_device();
        unsafe {
            vk_device.destroy_semaphore(self.vk_semaphore, None);
        }
    }
}
