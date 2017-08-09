//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::vk;
use std::{ptr, fmt};

use {DeviceRef, Backend, AshDevice};
use imp::{DeviceConfig, Fence, CommandDependencyTable, LlFence};
use super::NestedPassEncoder;
use super::encoder::EncoderState;
use super::cbpool::CommandBufferPool;
use super::mutex::ResourceMutex;

#[derive(Debug)]
pub(super) struct CommandPass<T: DeviceRef> {
    pub(super) internal_queue_index: usize,
    pub(super) buffer: vk::CommandBuffer,

    pub(super) wait_fences: Vec<(Fence<T>, core::PipelineStageFlags, core::AccessTypeFlags)>,
    pub(super) update_fences: Vec<(Fence<T>, core::PipelineStageFlags, core::AccessTypeFlags)>,
}

pub struct CommandBuffer<T: DeviceRef> {
    pub(super) data: Box<CommandBufferData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for CommandBuffer<T> => data
}

pub(super) struct CommandBufferData<T: DeviceRef> {
    pub(super) device_ref: T,
    pub(super) device_config: DeviceConfig,

    /// Vulkan command pool for each internal queue.
    pub(super) pools: ResourceMutex<LlFence<T>, CommandBufferPoolSet<T>>,

    pub(super) passes: Vec<CommandPass<T>>,
    pub(super) nested_encoder: NestedPassEncoder<T>,

    pub(super) encoder_state: EncoderState<T>,

    pub(super) dependency_table: CommandDependencyTable<T>,
}

/// Vulkan command pool for each internal queue.
#[derive(Debug)]
pub(super) struct CommandBufferPoolSet<T: DeviceRef>(Vec<CommandBufferPool>, T);

impl<T: DeviceRef> fmt::Debug for CommandBufferData<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CommandBufferData")
            .field("device_ref", &self.device_ref)
            .field("device_config", &self.device_config)
            .field("passes", &self.passes)
            .field("encoder_state", &self.encoder_state)
            .finish()
    }
}

impl<T: DeviceRef> Drop for CommandBufferPoolSet<T> {
    fn drop(&mut self) {
        for q_pool in self.0.drain(..) {
            unsafe {
                q_pool.destroy(&self.1);
            }
        }
    }
}

impl<T: DeviceRef> CommandBufferPoolSet<T> {
    pub fn reset(&mut self) {
        let device: &AshDevice = self.1.device();
        for pool in self.0.iter_mut() {
            unsafe {
                pool.reset(device);
            }
        }
    }
    pub fn get_mut(&mut self, iq: usize) -> &mut CommandBufferPool {
        &mut self.0[iq]
    }
}

impl<T: DeviceRef> Drop for CommandBuffer<T> {
    fn drop(&mut self) {
        use core::CommandBuffer;
        // FIXME: should we panic instead?
        self.wait_completion().unwrap();
    }
}

impl<T: DeviceRef> CommandBuffer<T> {
    pub(super) fn new(device_ref: &T, device_config: &DeviceConfig) -> core::Result<Self> {
        let mut pool_set = CommandBufferPoolSet(Vec::new(), device_ref.clone());
        // Create `CommandBufferPool`s
        for &(family, _) in device_config.queues.iter() {
            let info = vk::CommandPoolCreateInfo {
                s_type: vk::StructureType::CommandPoolCreateInfo,
                p_next: ptr::null(),
                flags: vk::COMMAND_POOL_CREATE_TRANSIENT_BIT,
                queue_family_index: family,
            };

            let pool = unsafe { CommandBufferPool::new(device_ref, &info)? };
            pool_set.0.push(pool);
        }

        let data = CommandBufferData {
            device_ref: device_ref.clone(),
            device_config: device_config.clone(),
            pools: ResourceMutex::new(pool_set, true),
            passes: Vec::new(),
            nested_encoder: NestedPassEncoder::new(),
            encoder_state: EncoderState::Initial,
            dependency_table: CommandDependencyTable::new(),
        };

        Ok(CommandBuffer { data: Box::new(data) })
    }

    /// Removes all command passes and returns all `vk::CommandBuffer`s to
    /// `pools`.
    pub(super) fn reset(&mut self) {
        let ref mut data = *self.data;
        data.pools.lock_host_write().reset();
        data.passes.clear();
    }

    pub(super) fn dependency_table(&mut self) -> Option<&mut CommandDependencyTable<T>> {
        Some(&mut self.data.dependency_table)
    }
}

impl<T: DeviceRef> core::CommandBuffer<Backend<T>> for CommandBuffer<T> {
    fn state(&self) -> core::CommandBufferState {
        match self.data.encoder_state {
            EncoderState::Initial => core::CommandBufferState::Initial,
            EncoderState::Error(_) => core::CommandBufferState::Error,
            EncoderState::Invalid => unreachable!(),
            EncoderState::End => core::CommandBufferState::Executable,
            EncoderState::Submitted => {
                if self.data.pools.is_host_writable() {
                    core::CommandBufferState::Completed
                } else {
                    core::CommandBufferState::Pending
                }
            }
            _ => core::CommandBufferState::Recording,
        }
    }
    fn wait_completion(&self) -> core::Result<()> {
        self.data.pools.wait_host_writable();
        Ok(())
    }
}

impl<T: DeviceRef> core::Marker for CommandBuffer<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}
