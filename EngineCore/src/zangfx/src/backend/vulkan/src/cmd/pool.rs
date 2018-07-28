//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdPool` for Vulkan.
use std::sync::Arc;

use base;
use base::Result;

use super::buffer::CmdBuffer;
use super::bufferpool::VkCmdBufferPool;
use super::queue::Scheduler;
use device::DeviceRef;

/// Implementation of `CmdPool` for Vulkan.
#[derive(Debug)]
pub struct CmdPool {
    device: DeviceRef,
    vk_cmd_buffer_pool: VkCmdBufferPool,
    scheduler: Arc<Scheduler>,
}

// zangfx_impl_object! { CmdPool: base::CmdPool, ::Debug }

impl CmdPool {
    pub(super) fn new(
        device: DeviceRef,
        scheduler: Arc<Scheduler>,
        queue_family_index: u32,
        num_cbs: usize,
    ) -> Result<Self> {
        let vk_cmd_buffer_pool = VkCmdBufferPool::new(device, queue_family_index, num_cbs)?;
        Ok(Self {
            device,
            vk_cmd_buffer_pool,
            scheduler,
        })
    }
    /* }

impl base::CmdPool for CmdPool { */
    unsafe fn new_cmd_buffer(&mut self) -> Result<Box<base::CmdBuffer>> {
        CmdBuffer::new(
            self.device,
            self.vk_cmd_buffer_pool.new_cmd_buffer()?,
            Arc::clone(&self.scheduler),
        ).map(|x| Box::new(x) as _)
    }
}
