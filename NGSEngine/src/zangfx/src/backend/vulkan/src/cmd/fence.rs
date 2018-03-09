//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Fence` for Vulkan.
//!
//! ZanGFX fences are implemented using `VkEvent`s. Fence update/wait operations
//! are translated to event signal/wait operations with some exceptions.
//!
//! # Fence operations in render passes
//!
//! Vulkan's render passes automatically insert memory barriers, making
//! inserting barriers via `VkEvent`s entirely unnecessary. However, we must be
//! careful about the following points:
//!
//!  - Other passes expect that `VkEvent` are signaled as usual, so we still
//!    have to signal `VkEvent`s in a render pass.
//!  - Fences still determine the execution ordering of command buffers.
//!
use ash::vk;
use ash::version::*;
use tokenlock::{TokenLock, TokenRef};
use refeq::RefEqArc;

use base;
use common::Result;
use device::DeviceRef;
use limits::DeviceTrait;

use utils::translate_generic_error_unwrap;
use cmd::queue::Item;

// TODO: recycle fences after use

/// Implementation of `Fence` for Vulkan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fence {
    data: RefEqArc<FenceData>,
}

zangfx_impl_handle! { Fence, base::Fence }

#[derive(Debug)]
struct FenceData {
    device: DeviceRef,
    vk_event: vk::Event,
    schedule: TokenLock<FenceScheduleData>,
}

#[derive(Debug)]
pub(super) struct FenceScheduleData {
    pub signaled: bool,
    pub waiting: Option<Box<Item>>,
}

impl Fence {
    pub(crate) unsafe fn new(device: DeviceRef, token_ref: TokenRef) -> Result<Self> {
        let info = vk::EventCreateInfo {
            s_type: vk::StructureType::EventCreateInfo,
            p_next: ::null(),
            flags: vk::EventCreateFlags::empty(),
        };

        let vk_device: &::AshDevice = device.vk_device();
        let mut vk_event = vk::Event::null();

        // Skip all event operations on MoltenVK -- Events are not supported.
        // It'll (probably) work without them thanks to Metal's automatic memory
        // barriers anyway.
        if !device.caps().info.traits.intersects(DeviceTrait::MoltenVK) {
            match vk_device.fp_v1_0().create_event(
                vk_device.handle(),
                &info,
                ::null(),
                &mut vk_event,
            ) {
                vk::Result::Success => {}
                e => return Err(translate_generic_error_unwrap(e)),
            }
        }

        Ok(Self {
            data: RefEqArc::new(FenceData {
                device,
                vk_event,
                schedule: TokenLock::new(
                    token_ref,
                    FenceScheduleData {
                        signaled: false,
                        waiting: None,
                    },
                ),
            }),
        })
    }

    pub fn vk_event(&self) -> vk::Event {
        self.data.vk_event
    }

    pub(super) fn schedule_data(&self) -> &TokenLock<FenceScheduleData> {
        &self.data.schedule
    }
}

impl Drop for FenceData {
    fn drop(&mut self) {
        let ref device = self.device;
        if !device.caps().info.traits.intersects(DeviceTrait::MoltenVK) {
            let vk_device: &::AshDevice = self.device.vk_device();
            unsafe {
                vk_device
                    .fp_v1_0()
                    .destroy_event(vk_device.handle(), self.vk_event, ::null());
            }
        }
    }
}
