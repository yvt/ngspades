//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use imp::{CommandBuffer, Framebuffer, SecondaryCommandBuffer};
use {DeviceRef, Backend};

impl<T: DeviceRef> core::CommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn begin_encoding(&mut self) {
        unimplemented!()
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
    /// `engine` must be `Universal`.
    fn begin_render_pass(&mut self, framebuffer: &Framebuffer<T>, engine: core::DeviceEngine) {
        unimplemented!()
    }
    fn begin_compute_pass(&mut self, engine: core::DeviceEngine) {
        unimplemented!()
    }
    fn begin_copy_pass(&mut self, engine: core::DeviceEngine) {
        unimplemented!()
    }
    fn make_secondary_command_buffer(&mut self) -> SecondaryCommandBuffer<T> {
        unimplemented!()
    }
    fn end_pass(&mut self) {
        unimplemented!()
    }
    fn begin_render_subpass(&mut self, contents: core::RenderPassContents) {
        unimplemented!()
    }
    fn end_render_subpass(&mut self) {
        unimplemented!()
    }
}
