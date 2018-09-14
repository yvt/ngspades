//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Semaphore` for Vulkan.
//!
//! ZanGFX semaphores are functionally equivalent to Vulkan's semaphores.
//!
use ash::version::*;
use ash::vk;
use refeq::RefEqArc;

use crate::device::DeviceRef;
use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_handle, zangfx_impl_object};

use crate::utils::translate_generic_error_unwrap;

/// Implementation of `SemaphoreBuilder` for Vulkan.
#[derive(Debug)]
pub struct SemaphoreBuilder {
    device: DeviceRef,
    raw: vk::Semaphore,
}

zangfx_impl_object! { SemaphoreBuilder: dyn base::SemaphoreBuilder, dyn (crate::Debug) }

impl SemaphoreBuilder {
    crate fn new(device: DeviceRef) -> Self {
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
            Ok(Semaphore::new(self.device.clone())?.into())
        } else {
            Ok(Semaphore {
                data: RefEqArc::new(SemaphoreData {
                    device: self.device.clone(),
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
    crate fn new(device: DeviceRef) -> Result<Self> {
        let info = vk::SemaphoreCreateInfo {
            s_type: vk::StructureType::SemaphoreCreateInfo,
            p_next: crate::null(),
            flags: vk::SemaphoreCreateFlags::empty(),
        };

        let vk_semaphore = unsafe {
            let vk_device: &crate::AshDevice = device.vk_device();
            vk_device.create_semaphore(&info, None)
        }.map_err(translate_generic_error_unwrap)?;

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
        let vk_device: &crate::AshDevice = self.device.vk_device();
        unsafe {
            vk_device.destroy_semaphore(self.vk_semaphore, None);
        }
    }
}
