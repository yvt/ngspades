//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;
use cgmath::Vector3;
use smallvec::SmallVec;

use imp::{CommandBuffer, DescriptorSet, PipelineLayout, ComputePipeline};
use {DeviceRef, Backend, AshDevice};

impl<T: DeviceRef> core::ComputeCommandEncoder<Backend<T>> for CommandBuffer<T> {
    // TODO: add strong references to given descriptor sets and pipelines, and so on
    // TODO: lock descriptor set update until command buffer execution is completed

    fn bind_compute_pipeline(&mut self, pipeline: &ComputePipeline<T>) {
        if self.encoder_error().is_some() {
            return;
        }

        let device: &AshDevice = self.data.device_ref.device();
        let buffer = self.expect_outside_render_pass().buffer;

        unsafe {
            device.cmd_bind_pipeline(buffer, vk::PipelineBindPoint::Compute, pipeline.handle());
        }
    }

    fn bind_compute_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout<T>,
        start_index: core::DescriptorSetBindingLocation,
        descriptor_sets: &[&DescriptorSet<T>],
        dynamic_offsets: &[u32],
    ) {
        if self.encoder_error().is_some() {
            return;
        }

        let device: &AshDevice = self.data.device_ref.device();
        let buffer = self.expect_outside_render_pass().buffer;

        let desc_sets: SmallVec<[_; 8]> = descriptor_sets.iter().map(|ds| ds.handle()).collect();
        unsafe {
            device.cmd_bind_descriptor_sets(
                buffer,
                vk::PipelineBindPoint::Compute,
                pipeline_layout.handle(),
                start_index as u32,
                &desc_sets,
                dynamic_offsets,
            );
        }
    }

    fn dispatch(&mut self, workgroup_count: Vector3<u32>) {
        if self.encoder_error().is_some() {
            return;
        }

        let device: &AshDevice = self.data.device_ref.device();
        let buffer = self.expect_outside_render_pass().buffer;

        unsafe {
            device.cmd_dispatch(
                buffer,
                workgroup_count.x,
                workgroup_count.y,
                workgroup_count.z,
            );
        }
    }
}
