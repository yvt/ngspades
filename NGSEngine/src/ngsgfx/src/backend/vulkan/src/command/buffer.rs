//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::vk;
use std::{ptr, fmt};

use {DeviceRef, Backend, AshDevice};
use imp::{DeviceConfig, Fence, CommandDependencyTable};
use super::NestedPassEncoder;
use super::encoder::EncoderState;
use super::cbpool::CommandBufferPool;

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
    pub(super) pools: Vec<CommandBufferPool>,

    pub(super) passes: Vec<CommandPass<T>>,
    pub(super) nested_encoder: NestedPassEncoder<T>,

    pub(super) encoder_state: EncoderState<T>,

    pub(super) dependency_table: CommandDependencyTable<T>,
}

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

impl<T: DeviceRef> Drop for CommandBufferData<T> {
    fn drop(&mut self) {
        for q_pool in self.pools.drain(..) {
            unsafe {
                q_pool.destroy(&self.device_ref);
            }
        }
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
        let mut data = CommandBufferData {
            device_ref: device_ref.clone(),
            device_config: device_config.clone(),
            pools: Vec::new(),
            passes: Vec::new(),
            nested_encoder: NestedPassEncoder::new(),
            encoder_state: EncoderState::Initial,
            dependency_table: CommandDependencyTable::new(),
        };

        // Create `CommandBufferPool`s
        //
        // (This is done after `CommandBufferData` was constructed so if an
        //  error should happen during the process, already created pools will
        //  be destroyed automatically by `CommandBufferData::drop`)
        for &(family, _) in data.device_config.queues.iter() {
            let info = vk::CommandPoolCreateInfo {
                s_type: vk::StructureType::CommandPoolCreateInfo,
                p_next: ptr::null(),
                flags: vk::COMMAND_POOL_CREATE_TRANSIENT_BIT,
                queue_family_index: family,
            };

            // Make a room for the new element
            //
            // (I don't want to leave `CommandBufferPool` in limbo in case of
            //  `Vec`'s allocation failure. `CommandBufferPool` does not
            //  implement `Drop`.)
            data.pools.reserve(1);

            let pool = unsafe { CommandBufferPool::new(device_ref, &info)? };
            data.pools.push(pool);
        }

        Ok(CommandBuffer { data: Box::new(data) })
    }

    /// Removes all command passes and returns all `vk::CommandBuffer`s to
    /// `pools`.
    pub(super) fn reset(&mut self) {
        let ref mut data = *self.data;
        let device: &AshDevice = data.device_ref.device();
        for pool in data.pools.iter_mut() {
            unsafe {
                pool.reset(device);
            }
        }
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
            EncoderState::End => {
                // TODO: return one of Executable, Pending, Completed, and Error
                unimplemented!()
            }
            _ => core::CommandBufferState::Recording,
        }
    }
    fn wait_completion(&self) -> core::Result<()> {
        unimplemented!()
    }
}

impl<T: DeviceRef> core::Marker for CommandBuffer<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
