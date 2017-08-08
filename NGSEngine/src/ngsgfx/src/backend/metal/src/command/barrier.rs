//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use imp::{Backend, CommandBuffer, Fence, SecondaryCommandBuffer};

impl core::BarrierCommandEncoder<Backend> for CommandBuffer {
    #[inline]
    fn wait_fence(&mut self, _: core::PipelineStageFlags, _: core::AccessTypeFlags, _: &Fence) {
        // no-op for now
    }

    #[inline]
    fn update_fence(&mut self, _: core::PipelineStageFlags, _: core::AccessTypeFlags, _: &Fence) {
        // no-op for now
    }

    #[inline]
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
}

impl core::BarrierCommandEncoder<Backend> for SecondaryCommandBuffer {
    #[inline]
    fn wait_fence(&mut self, _: core::PipelineStageFlags, _: core::AccessTypeFlags, _: &Fence) {
        // no-op for now
    }

    #[inline]
    fn update_fence(&mut self, _: core::PipelineStageFlags, _: core::AccessTypeFlags, _: &Fence) {
        // no-op for now
    }

    #[inline]
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
}
