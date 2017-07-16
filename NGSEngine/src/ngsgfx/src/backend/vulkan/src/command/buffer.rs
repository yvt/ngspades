//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::{self, vk};
use ash::version::DeviceV1_0;
use std::{ptr, fmt};

use {DeviceRef, Backend, translate_generic_error_unwrap, AshDevice};
use imp::{DeviceConfig, Fence};
use super::NestedPassEncoder;

#[derive(Debug)]
pub(super) struct QueuePool {
    vk_pool: vk::CommandPool, // destroyed by CommandBufferData::drop
    buffers: [(Vec<vk::CommandBuffer>, usize); 2],
}

impl QueuePool {
    fn new(vk_pool: vk::CommandPool) -> Self {
        Self {
            vk_pool,
            buffers: Default::default(),
        }
    }

    unsafe fn reset(&mut self, device: &AshDevice) {
        for &mut (_, ref mut used_count) in self.buffers.iter_mut() {
            *used_count = 0;
        }
        device.reset_command_pool(self.vk_pool, vk::CommandPoolResetFlags::empty());
    }

    unsafe fn get_buffer(&mut self, index: usize, device: &AshDevice) -> vk::CommandBuffer {
        let (ref mut reserve, ref mut used_count) = self.buffers[index];
        if *used_count == reserve.len() {
            let info = vk::CommandBufferAllocateInfo {
                s_type: vk::StructureType::CommandBufferAllocateInfo,
                p_next: ptr::null(),
                command_pool: self.vk_pool,
                level: if index == 0 {
                    vk::CommandBufferLevel::Primary
                } else {
                    vk::CommandBufferLevel::Secondary
                },
                command_buffer_count: 1,
            };
            let buffer = device.allocate_command_buffers(&info).expect(
                "command buffer allocation failed (sorry)",
            )
                [0]; // TODO: handle this error
            reserve.push(buffer);
        }
        *used_count += 1;
        reserve[*used_count - 1]
    }

    pub(super) unsafe fn get_primary_buffer(&mut self, device: &AshDevice) -> vk::CommandBuffer {
        self.get_buffer(0, device)
    }
    pub(super) unsafe fn get_secondary_buffer(&mut self, device: &AshDevice) -> vk::CommandBuffer {
        self.get_buffer(1, device)
    }
}

#[derive(Debug)]
pub(super) struct CommandPass<T: DeviceRef> {
    pub(super) internal_queue_index: usize,
    pub(super) buffer: vk::CommandBuffer,

    pub(super) wait_fences: Vec<(Fence<T>, core::PipelineStageFlags, core::AccessTypeFlags)>,
    pub(super) update_fences: Vec<(Fence<T>, core::PipelineStageFlags, core::AccessTypeFlags)>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum EncoderState {
    NoPass,

    RenderPrologue,
    RenderSubpassInline,
    RenderSubpassScb,
    RenderEpilogue,
    Compute,
    Copy,

    End,
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
    pub(super) pools: Vec<QueuePool>,

    pub(super) passes: Vec<CommandPass<T>>,
    pub(super) nested_encoder: NestedPassEncoder<T>,

    pub(super) encoder_state: EncoderState,
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
        let device: &AshDevice = self.device_ref.device();
        for q_pool in self.pools.iter() {
            // this frees all command buffers allocated from it automatically
            unsafe {
                device.destroy_command_pool(q_pool.vk_pool, self.device_ref.allocation_callbacks())
            };
        }
    }
}

impl<T: DeviceRef> Drop for CommandBuffer<T> {
    fn drop(&mut self) {
        use core::CommandBuffer;
        // FIXME: should we panic instead?
        self.wait_completion();
    }
}

impl<T: DeviceRef> CommandBuffer<T> {
    pub(super) fn new(device_ref: &T, device_config: &DeviceConfig) -> core::Result<Self> {
        let device_config = device_config.clone();
        let device: &AshDevice = device_ref.device();
        let mut data = CommandBufferData {
            device_ref: device_ref.clone(),
            device_config,
            pools: Vec::new(),
            passes: Vec::new(),
            nested_encoder: NestedPassEncoder::new(),
            encoder_state: EncoderState::NoPass,
        };
        let mut info = vk::CommandPoolCreateInfo {
            s_type: vk::StructureType::CommandPoolCreateInfo,
            p_next: ptr::null(),
            flags: vk::COMMAND_POOL_CREATE_TRANSIENT_BIT,
            queue_family_index: 0, // set it later
        };
        for &(family, _) in data.device_config.queues.iter() {
            info.queue_family_index = family;
            let handle = unsafe {
                device.create_command_pool(&info, device_ref.allocation_callbacks())
            }.map_err(translate_generic_error_unwrap)?;
            data.pools.push(QueuePool::new(handle));
        }
        Ok(CommandBuffer { data: Box::new(data) })
    }

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

    pub(super) fn expect_pass(&self) -> &CommandPass<T> {
        assert_ne!(self.data.encoder_state, EncoderState::NoPass);
        &self.data.passes[self.data.passes.len() - 1]
    }

    pub(super) fn expect_pass_mut(&mut self) -> &mut CommandPass<T> {
        let ref mut data = *self.data;
        assert_ne!(data.encoder_state, EncoderState::NoPass);

        let i = data.passes.len() - 1;
        &mut data.passes[i]
    }

    pub(super) fn expect_action_pass_mut(&mut self) -> &mut CommandPass<T> {
        match self.data.encoder_state {
            EncoderState::RenderSubpassInline |
            EncoderState::Compute |
            EncoderState::Copy => self.expect_pass_mut(),
            _ => unreachable!(),
        }
    }

    pub(super) fn expect_outside_render_pass(&self) -> &CommandPass<T> {
        match self.data.encoder_state {
            EncoderState::RenderEpilogue |
            EncoderState::RenderPrologue |
            EncoderState::Compute |
            EncoderState::Copy => self.expect_pass(),
            _ => unreachable!(),
        }
    }

    pub(super) fn expect_render_subpass_inline(&self) -> &CommandPass<T> {
        assert_eq!(self.data.encoder_state, EncoderState::RenderSubpassInline);
        self.expect_pass()
    }

    pub(super) fn expect_render_subpass_scb(&self) -> &CommandPass<T> {
        assert_eq!(self.data.encoder_state, EncoderState::RenderSubpassScb);
        self.expect_pass()
    }
}

impl<T: DeviceRef> core::CommandBuffer<Backend<T>> for CommandBuffer<T> {
    fn state(&self) -> core::CommandBufferState {
        unimplemented!()
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
