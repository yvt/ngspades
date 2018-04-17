//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::vk;
use ash::version::DeviceV1_0;
use std::ptr;

use {DeviceRef, translate_generic_error_unwrap, AshDevice};

/// Internal pool for Vulkan command buffers.
///
/// Command buffers are allocated on a `vk::CommandPool`.
/// This must be destroyed explicitly by `destroy`.
#[derive(Debug)]
pub(super) struct CommandBufferPool {
    vk_pool: vk::CommandPool, // destroyed by CommandBufferData::drop
    buffers: [(Vec<vk::CommandBuffer>, usize); 2],
}

impl CommandBufferPool {
    pub unsafe fn new<T: DeviceRef>(
        device_ref: &T,
        info: &vk::CommandPoolCreateInfo,
    ) -> core::Result<Self> {
        let vk_pool = device_ref
            .device()
            .create_command_pool(info, device_ref.allocation_callbacks())
            .map_err(translate_generic_error_unwrap)?;

        Ok(Self {
            vk_pool,
            buffers: Default::default(),
        })
    }

    /// Destroys this `CommandBufferPool`.
    ///
    /// All command buffers allocated from this pool will be freed.
    ///
    /// `device_ref` must be the same one as used to create this `CommandBufferPool`.
    pub unsafe fn destroy<T: DeviceRef>(self, device_ref: &T) {
        device_ref.device().destroy_command_pool(
            self.vk_pool,
            device_ref.allocation_callbacks(),
        );
    }

    /// Reset all command buffers allocated from this pool.
    pub unsafe fn reset(&mut self, device: &AshDevice) {
        for &mut (_, ref mut used_count) in self.buffers.iter_mut() {
            *used_count = 0;
        }
        device
            .reset_command_pool(self.vk_pool, vk::CommandPoolResetFlags::empty())
            .unwrap(); // TODO: handle this error
    }

    unsafe fn get_buffer(
        &mut self,
        index: usize,
        device: &AshDevice,
    ) -> core::Result<vk::CommandBuffer> {
        let (ref mut reserve, ref mut used_count) = self.buffers[index];
        if *used_count == reserve.len() {
            // allocate extra buffers to avoid frequent allocations
            let extend_count = 1 + (reserve.len() >> 2);
            reserve.reserve(extend_count);

            let info = vk::CommandBufferAllocateInfo {
                s_type: vk::StructureType::CommandBufferAllocateInfo,
                p_next: ptr::null(),
                command_pool: self.vk_pool,
                level: if index == 0 {
                    vk::CommandBufferLevel::Primary
                } else {
                    vk::CommandBufferLevel::Secondary
                },
                command_buffer_count: extend_count as u32,
            };
            let buffers = device.allocate_command_buffers(&info).map_err(
                translate_generic_error_unwrap,
            )?;

            reserve.extend(buffers);
        }
        *used_count += 1;
        Ok(reserve[*used_count - 1])
    }

    pub unsafe fn get_primary_buffer(
        &mut self,
        device: &AshDevice,
    ) -> core::Result<vk::CommandBuffer> {
        self.get_buffer(0, device)
    }
    pub unsafe fn get_secondary_buffer(
        &mut self,
        device: &AshDevice,
    ) -> core::Result<vk::CommandBuffer> {
        self.get_buffer(1, device)
    }
}
