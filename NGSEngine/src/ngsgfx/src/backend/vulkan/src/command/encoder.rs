//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;
use std::{ptr, mem};

use imp::{CommandBuffer, Framebuffer, SecondaryCommandBuffer};
use {DeviceRef, Backend, AshDevice, translate_access_type_flags, translate_pipeline_stage_flags,
     translate_generic_error_unwrap};
use super::{NestedPassEncoder, CommandPass};
use super::barrier::VkResourceBarrier;

#[derive(Debug)]
pub(super) enum EncoderState<T: DeviceRef> {
    /// `CommandBuffer` was just created, or is awaiting the next encoding.
    Initial,

    NoPass,

    RenderPrologue(RenderPassState<T>),
    RenderSubpassInline(RenderPassState<T>),
    RenderSubpassScb(RenderPassState<T>),
    RenderPassIntermission(RenderPassState<T>),
    RenderEpilogue,
    Compute,
    Copy,

    End,

    Submitted,

    /// An error occured while encoding some commands.
    ///
    /// This error will be reported upon submission, via CB state,
    /// or whatever. This can be reset with `begin_encoding`.
    ///
    /// All commands encoded during this state will be ignored.
    Error(core::GenericError),

    /// An intermediate state. Not meant to be visible from the outside.
    Invalid,
}

#[derive(Debug)]
pub(super) struct RenderPassState<T: DeviceRef> {
    framebuffer: Framebuffer<T>,

    /// The current subpass index.
    /// Must be less than `framebuffer.num_subpasses()`.
    subpass: usize,
}

impl<T: DeviceRef> CommandBuffer<T> {
    pub(super) fn encoder_error(&self) -> Option<core::GenericError> {
        match self.data.encoder_state {
            EncoderState::Error(err) => Some(err),
            _ => None,
        }
    }

    pub(super) fn expect_recording_no_pass(&self) {
        match self.data.encoder_state {
            EncoderState::NoPass => {}
            _ => panic!("bad state"),
        }
    }

    /// Return the internal queue index of the currently active command pass.
    pub fn active_internal_queue_index(&self) -> Option<usize> {
        match self.data.encoder_state {
            EncoderState::NoPass | EncoderState::End => None,
            _ => Some(self.expect_pass().internal_queue_index),
        }
    }

    /// Return the `vk::CommandBuffer` of the currently active command pass.
    pub fn active_command_buffer(&self) -> Option<vk::CommandBuffer> {
        match self.data.encoder_state {
            EncoderState::NoPass | EncoderState::End => None,
            _ => Some(self.expect_pass().buffer),
        }
    }

    pub(super) fn expect_pass(&self) -> &CommandPass<T> {
        match self.data.encoder_state {
            EncoderState::NoPass | EncoderState::End => panic!("bad state"),
            _ => &self.data.passes[self.data.passes.len() - 1],
        }
    }

    pub(super) fn expect_pass_mut(&mut self) -> &mut CommandPass<T> {
        match self.data.encoder_state {
            EncoderState::NoPass | EncoderState::End => panic!("bad state"),
            _ => {
                let ref mut data = self.data;
                let i = data.passes.len() - 1;
                &mut data.passes[i]
            }
        }
    }

    pub(super) fn expect_action_pass_mut(&mut self) -> &mut CommandPass<T> {
        match self.data.encoder_state {
            EncoderState::RenderSubpassInline { .. } |
            EncoderState::Compute |
            EncoderState::Copy => self.expect_pass_mut(),
            _ => panic!("bad state"),
        }
    }

    pub(super) fn expect_outside_render_pass(&self) -> &CommandPass<T> {
        match self.data.encoder_state {
            EncoderState::RenderEpilogue |
            EncoderState::RenderPrologue { .. } |
            EncoderState::Compute |
            EncoderState::Copy => self.expect_pass(),
            _ => panic!("bad state"),
        }
    }

    pub(super) fn expect_render_subpass_inline(&self) -> &CommandPass<T> {
        match self.data.encoder_state {
            EncoderState::RenderSubpassInline { .. } => self.expect_pass(),
            _ => panic!("bad state"),
        }
    }

    fn begin_pass_internal(&mut self, engine: core::DeviceEngine) -> core::Result<()> {
        self.expect_recording_no_pass();

        let ref mut data = *self.data;
        let device: &AshDevice = data.device_ref.device();
        let iq = data.device_config
            .engine_queue_mappings
            .internal_queue_for_engine(engine)
            .unwrap();
        let buffer = unsafe {
            data.pools
                .lock_host_write()
                .get_mut(iq)
                .get_primary_buffer(device)?
        };

        unsafe {
            device
                .begin_command_buffer(
                    buffer,
                    &vk::CommandBufferBeginInfo {
                        s_type: vk::StructureType::CommandBufferBeginInfo,
                        p_next: ptr::null(),
                        flags: vk::COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                        p_inheritance_info: ptr::null(),
                    },
                )
                .unwrap(); // TODO: handle this error
        }

        data.passes.push(CommandPass {
            internal_queue_index: iq,
            buffer,
            wait_fences: Vec::new(),
            update_fences: Vec::new(),
        });

        Ok(())
    }
}

impl<T: DeviceRef> core::CommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn begin_encoding(&mut self) {
        use core::CommandBuffer;

        assert!(
            [
                core::CommandBufferState::Initial,
                core::CommandBufferState::Completed,
                core::CommandBufferState::Error,
            ].contains(&self.state())
        );

        self.reset();
        self.data.encoder_state = EncoderState::NoPass;
    }
    fn end_encoding(&mut self) -> core::Result<()> {
        if let Some(err) = self.encoder_error() {
            return Err(err);
        }

        self.expect_recording_no_pass();
        self.data.encoder_state = EncoderState::End;
        Ok(())
    }
    fn acquire_resource(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        from_engine: core::DeviceEngine,
        resource: &core::SubresourceWithLayout<Backend<T>>,
    ) {
        if self.encoder_error().is_some() {
            return;
        }

        let &CommandPass {
            internal_queue_index,
            buffer,
            ..
        } = self.expect_outside_render_pass();
        let ref mut data = self.data;
        let device: &AshDevice = data.device_ref.device();
        let barrier = if from_engine == core::DeviceEngine::Host {
            VkResourceBarrier::translate(
                resource,
                vk::ACCESS_HOST_READ_BIT | vk::ACCESS_HOST_WRITE_BIT,
                translate_access_type_flags(access),
                vk::VK_QUEUE_FAMILY_IGNORED,
                vk::VK_QUEUE_FAMILY_IGNORED,
            )
        } else {
            let another_iq_index = data.device_config
                .engine_queue_mappings
                .internal_queue_for_engine(from_engine)
                .unwrap();

            VkResourceBarrier::translate(
                resource,
                vk::AccessFlags::empty(),
                translate_access_type_flags(access),
                data.device_config.queues[another_iq_index].0,
                data.device_config.queues[internal_queue_index].0,
            )
        };
        unsafe {
            device.cmd_pipeline_barrier(
                buffer,
                if from_engine == core::DeviceEngine::Host {
                    vk::PIPELINE_STAGE_HOST_BIT
                } else {
                    // FIXME: this is over-conservative
                    vk::PIPELINE_STAGE_ALL_COMMANDS_BIT
                },
                translate_pipeline_stage_flags(stage),
                vk::DependencyFlags::empty(),
                &[],
                barrier.buffer_memory_barriers(),
                barrier.image_memory_barriers(),
            );
        }
    }
    fn release_resource(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        to_engine: core::DeviceEngine,
        resource: &core::SubresourceWithLayout<Backend<T>>,
    ) {
        if self.encoder_error().is_some() {
            return;
        }

        let &CommandPass {
            internal_queue_index,
            buffer,
            ..
        } = self.expect_outside_render_pass();
        let ref mut data = self.data;
        let device: &AshDevice = data.device_ref.device();
        let barrier = if to_engine == core::DeviceEngine::Host {
            VkResourceBarrier::translate(
                resource,
                translate_access_type_flags(access),
                vk::ACCESS_HOST_READ_BIT | vk::ACCESS_HOST_WRITE_BIT,
                vk::VK_QUEUE_FAMILY_IGNORED,
                vk::VK_QUEUE_FAMILY_IGNORED,
            )
        } else {
            let another_iq_index = data.device_config
                .engine_queue_mappings
                .internal_queue_for_engine(to_engine)
                .unwrap();

            VkResourceBarrier::translate(
                resource,
                translate_access_type_flags(access),
                vk::AccessFlags::empty(),
                data.device_config.queues[internal_queue_index].0,
                data.device_config.queues[another_iq_index].0,
            )
        };
        unsafe {
            device.cmd_pipeline_barrier(
                buffer,
                translate_pipeline_stage_flags(stage),
                if to_engine == core::DeviceEngine::Host {
                    vk::PIPELINE_STAGE_HOST_BIT
                } else {
                    // FIXME: this is over-conservative
                    vk::PIPELINE_STAGE_ALL_COMMANDS_BIT
                },
                vk::DependencyFlags::empty(),
                &[],
                barrier.buffer_memory_barriers(),
                barrier.image_memory_barriers(),
            );
        }
    }

    fn begin_render_pass(&mut self, framebuffer: &Framebuffer<T>, engine: core::DeviceEngine) {
        assert_eq!(engine, core::DeviceEngine::Universal);

        if self.encoder_error().is_some() {
            return;
        }

        self.dependency_table().unwrap().insert_framebuffer(
            framebuffer,
        );

        if let Err(err) = self.begin_pass_internal(engine) {
            self.data.encoder_state = EncoderState::Error(err);
            return;
        }

        let rps = RenderPassState {
            framebuffer: framebuffer.clone(),
            subpass: 0,
        };
        self.data.encoder_state = EncoderState::RenderPrologue(rps);

        // Vulkan render pass is started when the first subpass was started
    }
    fn begin_compute_pass(&mut self, engine: core::DeviceEngine) {
        assert!([core::DeviceEngine::Compute, core::DeviceEngine::Copy].contains(&engine));

        if self.encoder_error().is_some() {
            return;
        }

        if let Err(err) = self.begin_pass_internal(engine) {
            self.data.encoder_state = EncoderState::Error(err);
            return;
        }

        self.data.encoder_state = EncoderState::Compute;
    }
    fn begin_copy_pass(&mut self, engine: core::DeviceEngine) {
        assert!(
            [
                core::DeviceEngine::Universal,
                core::DeviceEngine::Compute,
                core::DeviceEngine::Copy,
            ].contains(&engine)
        );

        if self.encoder_error().is_some() {
            return;
        }

        if let Err(err) = self.begin_pass_internal(engine) {
            self.data.encoder_state = EncoderState::Error(err);
            return;
        }

        self.data.encoder_state = EncoderState::Copy;
    }
    fn make_secondary_command_buffer(&mut self) -> SecondaryCommandBuffer<T> {
        if self.encoder_error().is_some() {
            return self.data
                .nested_encoder
                .make_noop_secondary_command_buffer();
        }

        let ref mut data = *self.data;

        let result = match data.encoder_state {
            EncoderState::RenderSubpassScb(ref rp_state) => {
                let device_ref = &data.device_ref;
                let device: &AshDevice = device_ref.device();

                // Get a free secondary command buffer
                let ref mut nested_encoder: NestedPassEncoder<T> = data.nested_encoder;

                let univ_iq = data.device_config.engine_queue_mappings.universal;
                let ref mut univ_pool = data.pools.lock_host_write().get_mut(univ_iq);

                let scb =
                    nested_encoder.make_secondary_command_buffer(device_ref, &mut || unsafe {
                        univ_pool.get_secondary_buffer(device_ref.device())
                    });

                // Secondary command buffer creation might fail...
                match scb {
                    Ok(mut scb) => {
                        // Begin recording the command buffer
                        let scb_cb = scb.expect_active().unwrap().buffer;
                        let ref framebuffer: Framebuffer<T> = rp_state.framebuffer;

                        let ii = [
                            vk::CommandBufferInheritanceInfo {
                                s_type: vk::StructureType::CommandBufferInheritanceInfo,
                                p_next: ptr::null(),
                                render_pass: framebuffer.render_pass_handle(),
                                subpass: rp_state.subpass as u32,
                                framebuffer: framebuffer.handle(),
                                occlusion_query_enable: vk::VK_FALSE,
                                query_flags: vk::QueryControlFlags::empty(),
                                pipeline_statistics: vk::QueryPipelineStatisticFlags::empty(),
                            },
                        ];

                        let begin_info = vk::CommandBufferBeginInfo {
                            s_type: vk::StructureType::CommandBufferBeginInfo,
                            p_next: ptr::null(),
                            flags: vk::COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT |
                                vk::COMMAND_BUFFER_USAGE_RENDER_PASS_CONTINUE_BIT,
                            p_inheritance_info: ii.as_ptr(),
                        };

                        let begin_result = unsafe {
                            device.begin_command_buffer(scb_cb, &begin_info)
                        }.map_err(translate_generic_error_unwrap);

                        match begin_result {
                            Ok(()) => Ok(scb),
                            Err(err) => {
                                scb.release();
                                Err(err)
                            }
                        }
                    }
                    Err(err) => Err(err),
                }
            }
            _ => panic!("bad state"),
        };

        match result {
            Ok(scb) => scb,
            Err(err) => {
                data.encoder_state = EncoderState::Error(err);
                data.nested_encoder.make_noop_secondary_command_buffer()
            }
        }
    }
    fn end_pass(&mut self) {
        if self.encoder_error().is_some() {
            return;
        }

        match self.data.encoder_state {
            EncoderState::RenderEpilogue |
            EncoderState::Compute |
            EncoderState::Copy => {
                {
                    let buffer = self.expect_pass().buffer;
                    let device: &AshDevice = self.data.device_ref.device();

                    unsafe {
                        device.end_command_buffer(buffer).unwrap(); // TODO: handle this error
                    }
                }

                self.data.encoder_state = EncoderState::NoPass;
            }
            _ => panic!("bad state"),
        }
    }
    fn begin_render_subpass(&mut self, contents: core::RenderPassContents) {
        if self.encoder_error().is_some() {
            return;
        }

        match self.data.encoder_state {
            EncoderState::RenderPrologue(_) |
            EncoderState::RenderPassIntermission(_) => {}
            _ => panic!("bad state"),
        }

        let buffer = self.expect_pass().buffer;
        let ref mut data = *self.data;
        let device: &AshDevice = data.device_ref.device();

        let rp_state = match mem::replace(&mut data.encoder_state, EncoderState::Invalid) {
            EncoderState::RenderPrologue(rp_state) => {
                unsafe {
                    device.cmd_begin_render_pass(
                        buffer,
                        &rp_state.framebuffer.render_pass_begin_info(),
                        match contents {
                            core::RenderPassContents::Inline => vk::SubpassContents::Inline,
                            core::RenderPassContents::SecondaryCommandBuffers => {
                                vk::SubpassContents::SecondaryCommandBuffers
                            }
                        },
                    );
                }

                rp_state
            }
            EncoderState::RenderPassIntermission(rp_state) => {
                unsafe {
                    // ash does not expose some functions via `DeviceV1_0` so sometimes we need
                    // a direct access to `DeviceFnV1_0`
                    device.fp_v1_0().cmd_next_subpass(
                        buffer,
                        match contents {
                            core::RenderPassContents::Inline => vk::SubpassContents::Inline,
                            core::RenderPassContents::SecondaryCommandBuffers => {
                                vk::SubpassContents::SecondaryCommandBuffers
                            }
                        },
                    );
                }

                rp_state
            }
            _ => unreachable!(),
        };
        data.encoder_state = match contents {
            core::RenderPassContents::Inline => EncoderState::RenderSubpassInline(rp_state),
            core::RenderPassContents::SecondaryCommandBuffers => {
                data.nested_encoder.start();
                EncoderState::RenderSubpassScb(rp_state)
            }
        };
    }
    fn end_render_subpass(&mut self) {
        if self.encoder_error().is_some() {
            return;
        }

        match self.data.encoder_state {
            EncoderState::RenderSubpassInline(_) |
            EncoderState::RenderSubpassScb(_) => {}
            _ => panic!("bad state"),
        }

        let ref mut data = *self.data;
        let device_ref = &data.device_ref;
        let current_pass_index = data.passes.len() - 1;
        let ref mut current_pass = data.passes[current_pass_index];
        let device: &AshDevice = device_ref.device();

        let rp_state = match mem::replace(&mut data.encoder_state, EncoderState::Invalid) {
            EncoderState::RenderSubpassInline(rp_state) => Ok(rp_state),
            EncoderState::RenderSubpassScb(rp_state) => {
                let ref mut nested_encoder: NestedPassEncoder<T> = data.nested_encoder;
                let ref mut dependency_table = data.dependency_table;
                let mut result = Ok(rp_state);

                nested_encoder.end(|scbd| {
                    match scbd.result {
                        Ok(()) => {}
                        Err(err) => {
                            // Something went wrong while encoding this secondary
                            // command buffer
                            result = Err(err);
                        }
                    }
                    if result.is_err() {
                        return;
                    }

                    // Check if it is valid to wait on these fences from this
                    // internal queue
                    for fence in scbd.wait_fences.iter() {
                        fence.0.expect_waitable_by_iq(
                            current_pass.internal_queue_index,
                        );
                    }

                    current_pass.wait_fences.extend(scbd.wait_fences.drain(..));
                    current_pass.update_fences.extend(
                        scbd.update_fences.drain(..),
                    );

                    dependency_table.inherit(&mut scbd.dependency_table);

                    unsafe {
                        // TODO: minimize the number of calls
                        device.cmd_execute_commands(current_pass.buffer, &[scbd.buffer]);
                    }
                });

                result
            }
            _ => panic!("render subpass is not active"),
        };

        match rp_state {
            Ok(mut rp_state) => {
                rp_state.subpass += 1;

                if rp_state.subpass == rp_state.framebuffer.num_subpasses() {
                    data.encoder_state = EncoderState::RenderEpilogue;

                    // This was the last subpass.
                    unsafe {
                        device.cmd_end_render_pass(current_pass.buffer);
                    }
                } else {
                    data.encoder_state = EncoderState::RenderPassIntermission(rp_state);
                }
            }
            Err(err) => {
                data.encoder_state = EncoderState::Error(err);
            }
        }
    }
}
