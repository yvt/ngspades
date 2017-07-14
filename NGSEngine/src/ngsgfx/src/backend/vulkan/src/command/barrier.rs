//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use imp::{CommandBuffer, SecondaryCommandBuffer, Fence, Image};
use {DeviceRef, Backend};

impl<T: DeviceRef> core::BarrierCommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn wait_fence(&mut self, _: core::PipelineStageFlags, _: core::AccessTypeFlags, _: &Fence<T>) {
        unimplemented!()
    }

    fn update_fence(
        &mut self,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: &Fence<T>,
    ) {
        unimplemented!()
    }

    fn resource_barrier(
        &mut self,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: &core::SubresourceWithLayout<Backend<T>>,
    ) {
        unimplemented!()
    }
}

impl<T: DeviceRef> core::BarrierCommandEncoder<Backend<T>> for SecondaryCommandBuffer<T> {
    fn wait_fence(&mut self, _: core::PipelineStageFlags, _: core::AccessTypeFlags, _: &Fence<T>) {
        unimplemented!()
    }

    fn update_fence(
        &mut self,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: &Fence<T>,
    ) {
        unimplemented!()
    }

    fn resource_barrier(
        &mut self,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: &core::SubresourceWithLayout<Backend<T>>,
    ) {
        unimplemented!()
    }
}
