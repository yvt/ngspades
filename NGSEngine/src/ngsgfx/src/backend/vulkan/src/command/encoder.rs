//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::version::DeviceV1_0;

use imp::{CommandBuffer, Framebuffer, SecondaryCommandBuffer, DeviceConfig};
use {DeviceRef, Backend, AshDevice};
use super::{EncoderState, NestedPassEncoder};

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
        unimplemented!()
    }
    fn release_resource(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        to_engine: core::DeviceEngine,
        resource: &core::SubresourceWithLayout<Backend<T>>,
    ) {
        unimplemented!()
    }

    fn begin_render_pass(&mut self, framebuffer: &Framebuffer<T>, engine: core::DeviceEngine) {
        assert_eq!(engine, core::DeviceEngine::Universal);
        unimplemented!()
    }
    fn begin_compute_pass(&mut self, engine: core::DeviceEngine) {
        unimplemented!()
    }
    fn begin_copy_pass(&mut self, engine: core::DeviceEngine) {
        unimplemented!()
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
        unimplemented!()
    }
    fn begin_render_subpass(&mut self, contents: core::RenderPassContents) {

        unimplemented!()
    }
    fn end_render_subpass(&mut self) {
        match self.data.encoder_state {
            EncoderState::RenderSubpassInline => {}
            EncoderState::RenderSubpassScb => {
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
            }
            _ => panic!("render subpass is not active"),
        }
        unimplemented!()
    }
}
