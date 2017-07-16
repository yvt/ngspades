//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;
use std::ptr;

use imp::{CommandBuffer, Framebuffer, SecondaryCommandBuffer, DeviceConfig};
use {DeviceRef, Backend, AshDevice, translate_access_type_flags, translate_pipeline_stage_flags};
use super::{EncoderState, NestedPassEncoder, CommandPass};
use super::barrier::VkResourceBarrier;

impl<T: DeviceRef> CommandBuffer<T> {
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
        let device = data.device_ref.device();
        let iq = data.device_config
            .engine_queue_mappings
            .internal_queue_for_engine(engine)
            .unwrap();
        let buffer = unsafe { data.pools[iq].get_primary_buffer(device) };
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
    }
    fn end_encoding(&mut self) {
        unimplemented!()
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

        self.data.encoder_state = EncoderState::RenderPrologue { framebuffer: framebuffer.clone() };
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
        self.expect_render_subpass_scb();

        let ref mut data = *self.data;
        let device_ref = &data.device_ref;

        let ref mut nested_encoder: NestedPassEncoder<T> = data.nested_encoder;

        let univ_iq = data.device_config.engine_queue_mappings.universal;
        let ref mut univ_pool = data.pools[univ_iq];

        nested_encoder.make_secondary_command_buffer(device_ref, || unsafe {
            univ_pool.get_secondary_buffer(device_ref.device())
        })
    }
    fn end_pass(&mut self) {
        match self.data.encoder_state {
            EncoderState::RenderEpilogue |
            EncoderState::Compute |
            EncoderState::Copy => {
                self.data.encoder_state = EncoderState::NoPass;
            }
            _ => panic!("bad state"),
        }
    }
    fn begin_render_subpass(&mut self, contents: core::RenderPassContents) {
        let next_state = match self.data.encoder_state {
            EncoderState::RenderPrologue { ref framebuffer } => {
                let buffer = self.expect_pass().buffer;
                let device: &AshDevice = self.data.device_ref.device();
                unsafe {
                    device.cmd_begin_render_pass(
                        buffer,
                        &framebuffer.render_pass_begin_info(),
                        match contents {
                            core::RenderPassContents::Inline => vk::SubpassContents::Inline,
                            core::RenderPassContents::SecondaryCommandBuffers => {
                                vk::SubpassContents::SecondaryCommandBuffers
                            }
                        },
                    );
                }

                match contents {
                    core::RenderPassContents::Inline => EncoderState::RenderSubpassInline {
                        num_remaining_subpasses: framebuffer.num_subpasses(),
                    },
                    core::RenderPassContents::SecondaryCommandBuffers => {
                        EncoderState::RenderSubpassScb {
                            num_remaining_subpasses: framebuffer.num_subpasses(),
                        }
                    }
                }
            }
            EncoderState::RenderPassIntermission { num_remaining_subpasses } => {
                let buffer = self.expect_pass().buffer;
                let device: &AshDevice = self.data.device_ref.device();
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
                    core::RenderPassContents::Inline => EncoderState::RenderSubpassInline {
                        num_remaining_subpasses: num_remaining_subpasses - 1,
                    },
                    core::RenderPassContents::SecondaryCommandBuffers => {
                        EncoderState::RenderSubpassScb {
                            num_remaining_subpasses: num_remaining_subpasses - 1,
                        }
                    }
                }
            }
            _ => {
                panic!("bad state");
            }
        };
        self.data.encoder_state = next_state;
    }
    fn end_render_subpass(&mut self) {
        let num_remaining_subpasses = match self.data.encoder_state {
            EncoderState::RenderSubpassInline { num_remaining_subpasses } => {
                num_remaining_subpasses
            }
            EncoderState::RenderSubpassScb { num_remaining_subpasses } => {
                let ref mut data = *self.data;
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
                        device.cmd_execute_commands(current_pass.buffer, &[scbd.buffer]);
                    }
                });
                num_remaining_subpasses
            }
            _ => panic!("render subpass is not active"),
        };

        if num_remaining_subpasses == 0 {
            self.data.encoder_state = EncoderState::RenderEpilogue;
        } else {
            self.data.encoder_state =
                EncoderState::RenderPassIntermission { num_remaining_subpasses };
        }
    }
}
