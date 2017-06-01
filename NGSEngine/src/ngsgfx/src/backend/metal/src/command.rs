//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, metal};
use metal::NSObjectProtocol;
use enumflags::BitFlags;
use cgmath::Vector3;

use std::time::Duration;

use {ref_hash, OCPtr};
use imp::{Backend, Buffer, BufferView, ComputePipeline, DescriptorPool, DescriptorSet,
          DescriptorSetLayout, Fence, Framebuffer, GraphicsPipeline, Heap, Image, ImageView,
          PipelineLayout, RenderPass, Sampler, Semaphore, ShaderModule, StencilState};

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct CommandQueue {
    obj: OCPtr<metal::MTLCommandQueue>,
}

unsafe impl Send for CommandQueue {}

impl CommandQueue {
    pub(crate) unsafe fn from_raw(obj: metal::MTLCommandQueue) -> Self {
        Self { obj: OCPtr::from_raw(obj).unwrap() }
    }
}

impl core::CommandQueue<Backend> for CommandQueue {
    fn make_command_buffer(&self) -> core::Result<CommandBuffer> {
        unimplemented!()
    }

    fn wait_idle(&self) {
        unimplemented!()
    }

    fn submit_commands(&self,
                       submissions: &[&core::SubmissionInfo<Backend>],
                       fence: Option<&Fence>)
                       -> core::Result<()> {
        unimplemented!()
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct CommandBuffer {}

impl core::CommandBuffer<Backend> for CommandBuffer {
    fn state(&self) -> core::CommandBufferState {
        unimplemented!()
    }
    fn wait_completion(&self, timeout: Duration) -> core::Result<bool> {
        unimplemented!()
    }
}

impl core::CommandEncoder<Backend> for CommandBuffer {
    fn begin_encoding(&mut self) {
        unimplemented!()
    }

    fn end_encoding(&mut self) {
        unimplemented!()
    }

    fn begin_render_pass(&mut self, framebuffer: &Framebuffer) {
        unimplemented!()
    }

    fn end_render_pass(&mut self) {
        unimplemented!()
    }

    fn next_subpass(&mut self) {
        unimplemented!()
    }
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline) {
        unimplemented!()
    }
    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        unimplemented!()
    }
    fn set_depth_bias(&mut self, value: &Option<core::DepthBias>) {
        unimplemented!()
    }
    fn set_depth_bounds(&mut self, value: &Option<core::DepthBounds>) {
        unimplemented!()
    }
    fn set_stencil_state(&mut self, value: &StencilState) {
        unimplemented!()
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        unimplemented!()
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<i32>) {
        unimplemented!()
    }
    fn bind_descriptor_sets(&mut self,
                            pipeline_layout: &PipelineLayout,
                            start_index: usize,
                            descriptor_sets: &[DescriptorSet],
                            dynamic_offsets: &[u32]) {
        unimplemented!()
    }
    fn draw(&mut self,
            num_vertices: u32,
            num_instances: u32,
            start_vertex_index: u32,
            start_instance_index: u32) {
        unimplemented!()
    }
    fn draw_indexed(&mut self,
                    num_vertices: u32,
                    num_instances: u32,
                    start_vertex_index: u32,
                    index_offset: u32,
                    start_instance_index: u32) {
        unimplemented!()
    }
    fn bind_compute_pipeline(&mut self, pipeline: &ComputePipeline) {
        unimplemented!()
    }

    fn dispatch(&mut self, workgroup_count: Vector3<u32>) {
        unimplemented!()
    }
}
