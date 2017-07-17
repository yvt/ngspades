//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;
use std::{ptr, mem};

use imp::{CommandBuffer, Framebuffer, SecondaryCommandBuffer, DeviceConfig};
use {DeviceRef, Backend, AshDevice, translate_access_type_flags, translate_pipeline_stage_flags};
use super::{NestedPassEncoder, CommandPass};
use super::barrier::VkResourceBarrier;

#[derive(Debug)]
pub(super) enum EncoderState<T: DeviceRef> {
    NoPass,

    RenderPrologue(RenderPassState<T>),
    RenderSubpassInline(RenderPassState<T>),
    RenderSubpassScb(RenderPassState<T>),
    RenderPassIntermission(RenderPassState<T>),
    RenderEpilogue,
    Compute,
    Copy,

    End,

    /// An error occured while encoding some commands.
    ///
    /// This error will be reported upon submission or via CB state.
    Error(core::GenericError),

    /// Internal error
    Invalid,
}

#[derive(Debug)]
pub(super) struct RenderPassState<T: DeviceRef> {
    framebuffer: Framebuffer<T>,
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

    pub(super) fn expect_render_subpass_scb(&self) -> &CommandPass<T> {
        match self.data.encoder_state {
            EncoderState::RenderSubpassScb { .. } => self.expect_pass(),
            _ => panic!("bad state"),
        }
    }

    fn begin_pass_internal(&mut self, engine: core::DeviceEngine) {
        self.expect_recording_no_pass();

        let ref mut data = *self.data;
        let device: &AshDevice = data.device_ref.device();
        let iq = data.device_config
            .engine_queue_mappings
            .internal_queue_for_engine(engine)
            .unwrap();
        let buffer = unsafe { data.pools[iq].get_primary_buffer(device) };

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
    }
}

impl<T: DeviceRef> core::CommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn begin_encoding(&mut self) {
        use core::CommandBuffer;

        assert!(
            [
                core::CommandBufferState::Initial,
                core::CommandBufferState::Completed,
            ].contains(&self.state())
        );

        self.reset();
        self.data.encoder_state = EncoderState::NoPass;
    }
    fn end_encoding(&mut self) {
        self.expect_recording_no_pass();
        self.data.encoder_state = EncoderState::End;
    }
    fn acquire_resource(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        from_engine: core::DeviceEngine,
        resource: &core::SubresourceWithLayout<Backend<T>>,
    ) {
        let &CommandPass {
            internal_queue_index,
            buffer,
            ..
        } = self.expect_outside_render_pass();
        let ref mut data = self.data;
        let device: &AshDevice = data.device_ref.device();
        let another_iq_index = data.device_config
            .engine_queue_mappings
            .internal_queue_for_engine(from_engine)
            .unwrap();

        let barrier = VkResourceBarrier::translate(
            resource,
            vk::AccessFlags::empty(),
            translate_access_type_flags(access),
            data.device_config.queues[another_iq_index].0,
            data.device_config.queues[internal_queue_index].0,
        );
        unsafe {
            device.cmd_pipeline_barrier(
                buffer,
                vk::PipelineStageFlags::empty(),
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
        let &CommandPass {
            internal_queue_index,
            buffer,
            ..
        } = self.expect_outside_render_pass();
        let ref mut data = self.data;
        let device: &AshDevice = data.device_ref.device();
        let another_iq_index = data.device_config
            .engine_queue_mappings
            .internal_queue_for_engine(to_engine)
            .unwrap();

        let barrier = VkResourceBarrier::translate(
            resource,
            translate_access_type_flags(access),
            vk::AccessFlags::empty(),
            data.device_config.queues[internal_queue_index].0,
            data.device_config.queues[another_iq_index].0,
        );
        unsafe {
            device.cmd_pipeline_barrier(
                buffer,
                translate_pipeline_stage_flags(stage),
                vk::PipelineStageFlags::empty(),
                vk::DependencyFlags::empty(),
                &[],
                barrier.buffer_memory_barriers(),
                barrier.image_memory_barriers(),
            );
        }
    }

    fn begin_render_pass(&mut self, framebuffer: &Framebuffer<T>, engine: core::DeviceEngine) {
        assert_eq!(engine, core::DeviceEngine::Universal);
        self.begin_pass_internal(engine);

        let rps = RenderPassState {
            framebuffer: framebuffer.clone(),
            subpass: 0,
        };
        self.data.encoder_state = EncoderState::RenderPrologue(rps);
        let buffer = self.expect_pass().buffer;

        // Vulkan render pass is started when the first subpass was started
    }
    fn begin_compute_pass(&mut self, engine: core::DeviceEngine) {
        assert!([core::DeviceEngine::Compute, core::DeviceEngine::Copy].contains(&engine));
        self.begin_pass_internal(engine);
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
        self.begin_pass_internal(engine);
        self.data.encoder_state = EncoderState::Copy;
    }
    fn make_secondary_command_buffer(&mut self) -> SecondaryCommandBuffer<T> {
        let ref mut data = *self.data;

        match data.encoder_state {
            EncoderState::RenderSubpassScb(ref rp_state) => {
                let device_ref = &data.device_ref;
                let device: &AshDevice = device_ref.device();

                // Get a free secondary command buffer
                let ref mut nested_encoder: NestedPassEncoder<T> = data.nested_encoder;

                let univ_iq = data.device_config.engine_queue_mappings.universal;
                let ref mut univ_pool = data.pools[univ_iq];

                let scb = nested_encoder.make_secondary_command_buffer(device_ref, || unsafe {
                    univ_pool.get_secondary_buffer(device_ref.device())
                });

                // Begin recording the command buffer
                let scb_cb = scb.exepct_active().buffer;
                let ref framebuffer: Framebuffer<T> = rp_state.framebuffer;

                unsafe {
                    device
                        .begin_command_buffer(
                            scb_cb,
                            &vk::CommandBufferBeginInfo {
                                s_type: vk::StructureType::CommandBufferBeginInfo,
                                p_next: ptr::null(),
                                flags: vk::COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT |
                                    vk::COMMAND_BUFFER_USAGE_RENDER_PASS_CONTINUE_BIT,
                                p_inheritance_info: [
                                    vk::CommandBufferInheritanceInfo {
                                        s_type: vk::StructureType::CommandBufferInheritanceInfo,
                                        p_next: ptr::null(),
                                        render_pass: framebuffer.render_pass_handle(),
                                        subpass: rp_state.subpass as u32,
                                        framebuffer: framebuffer.handle(),
                                        occlusion_query_enable: vk::VK_FALSE,
                                        query_flags: vk::QueryControlFlags::empty(),
                                        pipeline_statistics:
                                            vk::QueryPipelineStatisticFlags::empty(),
                                    },
                                ].as_ptr(),
                            },
                        )
                        .unwrap(); // TODO: handle this error
                }

                scb
            }
            _ => panic!("bad state"),
        }
    }
    fn end_pass(&mut self) {
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
        match self.data.encoder_state {
            EncoderState::RenderPrologue(_) |
            EncoderState::RenderPassIntermission(_) => {}
            _ => panic!("bad state"),
        }

        let buffer = self.expect_pass().buffer;
        let ref mut data = *self.data;
        let device: &AshDevice = data.device_ref.device();

        let next_state = match mem::replace(&mut data.encoder_state, EncoderState::Invalid) {
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

                match contents {
                    core::RenderPassContents::Inline => EncoderState::RenderSubpassInline(rp_state),
                    core::RenderPassContents::SecondaryCommandBuffers => {
                        EncoderState::RenderSubpassScb(rp_state)
                    }
                }
            }
            EncoderState::RenderPassIntermission(rp_state) => {
                unsafe {
                    unimplemented!();
                    // why does not ash have `cmd_next_subpass`
                    /*device.cmd_next_subpass(
                        buffer,
                        match contents {
                            core::RenderPassContents::Inline => vk::SubpassContents::Inline,
                            core::RenderPassContents::SecondaryCommandBuffers => vk::SubpassContents::SecondaryCommandBuffers,
                        },
                    );*/
                }

                match contents {
                    core::RenderPassContents::Inline => EncoderState::RenderSubpassInline(rp_state),
                    core::RenderPassContents::SecondaryCommandBuffers => {
                        EncoderState::RenderSubpassScb(rp_state)
                    }
                }
            }
            _ => unreachable!(),
        };
        data.encoder_state = next_state;
    }
    fn end_render_subpass(&mut self) {
        match self.data.encoder_state {
            EncoderState::RenderSubpassInline(_) |
            EncoderState::RenderSubpassScb(_) => {}
            _ => panic!("bad state"),
        }

        let ref mut data = *self.data;
        let mut rp_state = match mem::replace(&mut data.encoder_state, EncoderState::Invalid) {
            EncoderState::RenderSubpassInline(rp_state) => rp_state,
            EncoderState::RenderSubpassScb(rp_state) => {
                let device_ref = &data.device_ref;

                let ref mut nested_encoder: NestedPassEncoder<T> = data.nested_encoder;
                let current_pass_index = data.passes.len() - 1;
                let ref mut current_pass = data.passes[current_pass_index];
                let device: &AshDevice = device_ref.device();
                nested_encoder.end(|scbd| {
                    current_pass.wait_fences.extend(scbd.wait_fences.drain(..));
                    current_pass.update_fences.extend(
                        scbd.update_fences.drain(..),
                    );

                    unsafe {
                        // TODO: minimize the number of calls
                        device.cmd_execute_commands(current_pass.buffer, &[scbd.buffer]);
                    }
                });
                rp_state
            }
            _ => panic!("render subpass is not active"),
        };

        rp_state.subpass += 1;

        if rp_state.subpass == rp_state.framebuffer.num_subpasses() {
            data.encoder_state = EncoderState::RenderEpilogue;
        } else {
            data.encoder_state = EncoderState::RenderPassIntermission(rp_state);
        }
    }
}
