//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use cgmath::Vector3;

use imp::{CommandBuffer, DescriptorSet, PipelineLayout, ComputePipeline};
use {DeviceRef, Backend};

impl<T: DeviceRef> core::ComputeCommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn bind_compute_pipeline(&mut self, pipeline: &ComputePipeline<T>) {
        unimplemented!()
    }

    fn bind_compute_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout<T>,
        start_index: core::DescriptorSetBindingLocation,
        descriptor_sets: &[&DescriptorSet<T>],
        dynamic_offsets: &[u32],
    ) {
        unimplemented!()
    }

    fn dispatch(&mut self, workgroup_count: Vector3<u32>) {
        unimplemented!()
    }
}
