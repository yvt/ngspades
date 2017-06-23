//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, metal, OCPtr};

use cgmath::Vector3;

use imp::{Backend, CommandBuffer, ComputePipeline, PipelineLayout, DescriptorSet};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ComputeCommandEncoder {
    metal_encoder: OCPtr<metal::MTLComputeCommandEncoder>,
    pipeline: Option<ComputePipeline>,
}

impl ComputeCommandEncoder {
    pub fn new(metal_encoder: OCPtr<metal::MTLComputeCommandEncoder>) -> Self {
        Self {
            metal_encoder,
            pipeline: None,
        }
    }

    pub fn end_encoding(&mut self) {
        self.metal_encoder.end_encoding();
        self.pipeline = None;
    }

    pub fn metal_command_encoder(&self) -> metal::MTLCommandEncoder {
        **self.metal_encoder
    }

    fn expect_pipeline(&self) -> &ComputePipeline {
        self.pipeline.as_ref().expect("no pipeline")
    }

    fn bind_compute_pipeline(&mut self, pipeline: &ComputePipeline) {
        self.pipeline = Some(pipeline.clone());
        pipeline.bind_pipeline_state(*self.metal_encoder);
    }

    fn bind_compute_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout,
        start_index: usize,
        descriptor_sets: &[DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        unimplemented!()
    }

    fn dispatch(&mut self, workgroup_count: Vector3<u32>) {
        self.metal_encoder.dispatch_threadgroups(
            metal::MTLSize {
                width: workgroup_count.x as u64,
                height: workgroup_count.y as u64,
                depth: workgroup_count.z as u64,
            },
            self.expect_pipeline().threads_per_threadgroup(),
        );
    }
}

impl core::ComputeCommandEncoder<Backend> for CommandBuffer {
    fn bind_compute_pipeline(&mut self, pipeline: &ComputePipeline) {
        self.expect_compute_pipeline().bind_compute_pipeline(
            pipeline,
        );
    }

    fn bind_compute_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout,
        start_index: usize,
        descriptor_sets: &[DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        self.expect_compute_pipeline().bind_compute_descriptor_sets(
            pipeline_layout,
            start_index,
            descriptor_sets,
            dynamic_offsets,
        );
    }

    fn dispatch(&mut self, workgroup_count: Vector3<u32>) {
        self.expect_compute_pipeline().dispatch(workgroup_count);
    }
}
