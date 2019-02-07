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
use ash::version::*;
use ash::vk;
use refeq::RefEqArc;

use zangfx_base as base;
use zangfx_base::{zangfx_impl_handle, Result};

use crate::cmd::queue::Item;
use crate::device::DeviceRef;
use crate::limits::DeviceTraitFlags;
use crate::resstate;
use crate::utils::translate_generic_error_unwrap;

// TODO: recycle fences after use

/// Implementation of `Fence` for Vulkan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fence {
    data: RefEqArc<FenceData>,
}

zangfx_impl_handle! { Fence, base::FenceRef }

#[derive(Debug)]
struct FenceData {
    device: DeviceRef,
    vk_event: vk::Event,
    tracked_state: resstate::TrackedState<FenceScheduleData>,
}

#[derive(Debug)]
crate struct FenceScheduleData {
    /*
     * Command buffer scheduling - These fields are used by the scheduler to
     * determine the order in which command buffers are executed
     */
    /// Indicates whether this fence is signaled.
    crate signaled: bool,

    /// Command queue items waiting for this fence to be signaled.
    crate waiting: Option<Box<Item>>,

    /*
     * Command buffer patching - This field is used after the ommand buffer
     * execution order is determined.
     */
    /// If this fence has been signaled, this field indicates the source access
    /// type flags.
    crate src_access: Option<base::AccessTypeFlags>,
}

impl Fence {
    crate unsafe fn new(device: DeviceRef, queue_id: resstate::QueueId) -> Result<Self> {
        let info = vk::EventCreateInfo {
            s_type: vk::StructureType::EVENT_CREATE_INFO,
            p_next: crate::null(),
            flags: vk::EventCreateFlags::empty(),
        };

        let mut vk_event = vk::Event::null();

        // Skip all event operations on MoltenVK -- Events are not supported.
        // It'll (probably) work without them thanks to Metal's automatic memory
        // barriers anyway.
        if !device
            .caps()
            .info
            .traits
            .intersects(DeviceTraitFlags::MOLTEN_VK)
        {
            let vk_device: &crate::AshDevice = device.vk_device();
            match vk_device.fp_v1_0().create_event(
                vk_device.handle(),
                &info,
                crate::null(),
                &mut vk_event,
            ) {
                e if e == vk::Result::SUCCESS => {}
                e => return Err(translate_generic_error_unwrap(e)),
            }
        }

        Ok(Self {
            data: RefEqArc::new(FenceData {
                device,
                vk_event,
                tracked_state: resstate::TrackedState::new(
                    queue_id,
                    FenceScheduleData {
                        signaled: false,
                        waiting: None,
                        src_access: None,
                    },
                ),
            }),
        })
    }

    pub fn vk_event(&self) -> vk::Event {
        self.data.vk_event
    }
}

impl resstate::Resource for Fence {
    type State = FenceScheduleData;

    fn tracked_state(&self) -> &resstate::TrackedState<Self::State> {
        &self.data.tracked_state
    }
}

impl Drop for FenceData {
    fn drop(&mut self) {
        let ref device = self.device;
        if !device
            .caps()
            .info
            .traits
            .intersects(DeviceTraitFlags::MOLTEN_VK)
        {
            let vk_device: &crate::AshDevice = self.device.vk_device();
            unsafe {
                vk_device
                    .fp_v1_0()
                    .destroy_event(vk_device.handle(), self.vk_event, crate::null());
            }
        }
    }
}
