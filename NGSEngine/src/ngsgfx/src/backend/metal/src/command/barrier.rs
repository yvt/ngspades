//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use imp::{Backend, CommandBuffer, Image};

impl core::BarrierCommandEncoder<Backend> for CommandBuffer {
    fn resource_barrier(
        &mut self,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: &core::SubresourceWithLayout<Backend>,
    ) {
        // no-op
    }

    fn acquire_resource(
        &mut self,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: core::DeviceEngine,
        _: &core::SubresourceWithLayout<Backend>,
    ) {
        // no-op
    }

    fn release_resource(
        &mut self,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: core::DeviceEngine,
        _: &core::SubresourceWithLayout<Backend>,
    ) {
        // no-op
    }

    fn image_layout_transition(
        &mut self,
        _: core::PipelineStageFlags,
        _: core::ImageLayout,
        _: core::PipelineStageFlags,
        _: core::ImageLayout,
        _: &Image,
    ) {
        // no-op
    }
}
