//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::version::*;
use ash::vk;
use parking_lot::Mutex;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;

use base::Result;

use device::DeviceRef;
use utils::translate_generic_error_unwrap;

/// Thread-safe command buffer pool. Maintains a fixed number of command
/// buffers.
#[derive(Debug)]
pub(super) struct VkCmdBufferPool {
    device: DeviceRef,
    data: Arc<Mutex<PoolData>>,
    cb_send: SyncSender<Option<vk::CommandBuffer>>,
}

/// Non-`Sync` data.
#[derive(Debug)]
struct PoolData {
    device: DeviceRef,
    vk_cmd_pool: vk::CommandPool,
    cb_recv: Receiver<Option<vk::CommandBuffer>>,
}

/// A command buffer allocated from `VkCmdBufferPool`. Returned to the original
/// pool on drop.
#[derive(Debug)]
pub(super) struct VkCmdBufferPoolItem {
    vk_cmd_buffer: vk::CommandBuffer,
    cb_send: SyncSender<Option<vk::CommandBuffer>>,
    data: Arc<Mutex<PoolData>>,
}

impl VkCmdBufferPool {
    pub fn new(device: DeviceRef, queue_family_index: u32, num_cbs: usize) -> Result<Self> {
        let (cb_send, cb_recv) = sync_channel(num_cbs);
        for _ in 0..num_cbs {
            cb_send.send(None).unwrap();
        }

        let vk_device = device.vk_device();
        let vk_cmd_pool = unsafe {
            vk_device.create_command_pool(
                &vk::CommandPoolCreateInfo {
                    s_type: vk::StructureType::CommandPoolCreateInfo,
                    p_next: ::null(),
                    flags: vk::COMMAND_POOL_CREATE_TRANSIENT_BIT
                        | vk::COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
                    queue_family_index,
                },
                None,
            )
        }.map_err(translate_generic_error_unwrap)?;

        Ok(Self {
            device,
            data: Arc::new(Mutex::new(PoolData {
                device,
                vk_cmd_pool,
                cb_recv,
            })),
            cb_send,
        })
    }

    /// Allocate an empty command buffer. Might block if there are an excessive
    /// number of outstanding command buffers.
    pub fn new_cmd_buffer(&self) -> Result<VkCmdBufferPoolItem> {
        use std::mem::drop;

        let vk_device = self.device.vk_device();

        let cb_send = self.cb_send.clone();

        let data = self.data.lock();
        let item = data.cb_recv.recv().unwrap();

        let result = unsafe {
            if let Some(vk_cmd_buffer) = item {
                // Reuse an existing command buffer.
                // `vkResetCommandBuffer` does not require an external
                // synchronization on the command pool, so we can release the
                // lock earlier.
                drop(data);
                vk_device
                    .reset_command_buffer(vk_cmd_buffer, vk::CommandBufferResetFlags::empty())
                    .map(|_| vk_cmd_buffer)
            } else {
                // Allocate a fresh command buffer
                vk_device
                    .allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                        s_type: vk::StructureType::CommandBufferAllocateInfo,
                        p_next: ::null(),
                        command_pool: data.vk_cmd_pool,
                        level: vk::CommandBufferLevel::Primary,
                        command_buffer_count: 1,
                    })
                    .map(|cbs| cbs[0])
            }
        }.map_err(translate_generic_error_unwrap);

        let vk_cmd_buffer = match result {
            Ok(cb) => cb,
            Err(e) => {
                // Requeue the item before returning
                self.cb_send.send(item).unwrap();
                return Err(e);
            }
        };

        Ok(VkCmdBufferPoolItem {
            vk_cmd_buffer,
            cb_send,
            data: Arc::clone(&self.data),
        })
    }
}

impl Drop for PoolData {
    fn drop(&mut self) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.destroy_command_pool(self.vk_cmd_pool, None);
        }
    }
}

impl VkCmdBufferPoolItem {
    pub fn vk_cmd_buffer(&self) -> vk::CommandBuffer {
        self.vk_cmd_buffer
    }
}

impl Drop for VkCmdBufferPoolItem {
    fn drop(&mut self) {
        // Return the command buffer to the pool. Do not care even if `send`
        // fails, in which case `VkCmdBufferPool` already have released the
        // pool as well as all command buffers.
        let _ = self.cb_send.send(Some(self.vk_cmd_buffer));
    }
}
