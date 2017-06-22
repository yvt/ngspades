//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, metal, OCPtr};

use imp::{Backend, CommandBuffer, StencilState, GraphicsPipeline, PipelineLayout,
    Buffer, DescriptorSet, SecondaryCommandBuffer};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RenderCommandEncoder {
    metal_encoder: OCPtr<metal::MTLRenderCommandEncoder>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GraphicsEncoderState {
    Inline(RenderCommandEncoder),
    SecondaryCommandBuffers(OCPtr<metal::MTLParallelRenderCommandEncoder>),
}

impl RenderCommandEncoder {
    pub fn new(metal_encoder: OCPtr<metal::MTLRenderCommandEncoder>) -> Self {
        Self { metal_encoder }
    }

    pub fn end_encoding(&self) {
        self.metal_encoder.end_encoding();
    }

    pub fn set_label(&self, label: &str) {
        self.metal_encoder.set_label(label);
    }

    fn bind_graphics_pipeline(&self, pipeline: &GraphicsPipeline) {
        self.metal_encoder.set_render_pipeline_state(
            unimplemented!(),
        );

        // TODO: don't forget to set static states!
    }
    fn set_blend_constants(&self, value: &[f32; 4]) {
        self.metal_encoder.set_blend_color(
            value[0],
            value[1],
            value[2],
            value[3],
        );
    }
    fn set_depth_bias(&self, value: Option<core::DepthBias>) {
        if let Some(value) = value {
            self.metal_encoder.set_depth_bias(
                value.constant_factor,
                value.slope_factor,
                value.clamp,
            );
        } else {
            self.metal_encoder.set_depth_bias(0f32, 0f32, 0f32);
        }
    }
    fn set_depth_bounds(&self, _: Option<core::DepthBounds>) {
        panic!("not supported");
    }
    fn set_stencil_state(&self, value: &StencilState) {
        unimplemented!()
    }
    fn set_viewport(&self, value: &core::Viewport) {
        unimplemented!()
    }
    fn set_scissor_rect(&self, value: &core::Rect2D<u32>) {
        unimplemented!()
    }
    fn bind_descriptor_sets(
        &self,
        pipeline_layout: &PipelineLayout,
        start_index: usize,
        descriptor_sets: &[DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        unimplemented!()
    }

    fn bind_vertex_buffers(&self, start_index: usize, buffers: &[(&Buffer, usize)]) {
        unimplemented!()
    }

    fn bind_index_buffer(&self, buffer: &Buffer, offset: usize, format: core::IndexFormat) {
        unimplemented!()
    }

    fn draw(
        &self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        start_instance_index: u32,
    ) {
        unimplemented!()
    }
    fn draw_indexed(
        &self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        index_offset: u32,
        start_instance_index: u32,
    ) {
        unimplemented!()
    }
}

impl core::RenderSubpassCommandEncoder<Backend> for CommandBuffer {
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline) {
        self.expect_graphics_pipeline().bind_graphics_pipeline(
            pipeline,
        )
    }
    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        self.expect_graphics_pipeline().set_blend_constants(value)
    }
    fn set_depth_bias(&mut self, value: Option<core::DepthBias>) {
        self.expect_graphics_pipeline().set_depth_bias(value)
    }
    fn set_depth_bounds(&mut self, value: Option<core::DepthBounds>) {
        self.expect_graphics_pipeline().set_depth_bounds(value)
    }
    fn set_stencil_state(&mut self, value: &StencilState) {
        self.expect_graphics_pipeline().set_stencil_state(value)
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        self.expect_graphics_pipeline().set_viewport(value)
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        self.expect_graphics_pipeline().set_scissor_rect(value)
    }
    fn bind_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout,
        start_index: usize,
        descriptor_sets: &[DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        self.expect_graphics_pipeline().bind_descriptor_sets(
            pipeline_layout,
            start_index,
            descriptor_sets,
            dynamic_offsets,
        )
    }

    fn bind_vertex_buffers(&mut self, start_index: usize, buffers: &[(&Buffer, usize)]) {
        self.expect_graphics_pipeline().bind_vertex_buffers(
            start_index,
            buffers,
        )
    }

    fn bind_index_buffer(&mut self, buffer: &Buffer, offset: usize, format: core::IndexFormat) {
        self.expect_graphics_pipeline().bind_index_buffer(
            buffer,
            offset,
            format,
        )
    }

    fn draw(
        &mut self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        start_instance_index: u32,
    ) {
        self.expect_graphics_pipeline().draw(
            num_vertices,
            num_instances,
            start_vertex_index,
            start_instance_index,
        )
    }
    fn draw_indexed(
        &mut self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        index_offset: u32,
        start_instance_index: u32,
    ) {
        self.expect_graphics_pipeline().draw_indexed(
            num_vertices,
            num_instances,
            start_vertex_index,
            index_offset,
            start_instance_index,
        )
    }
}

impl core::RenderSubpassCommandEncoder<Backend> for SecondaryCommandBuffer {
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline) {
        self.render_command_encoder().bind_graphics_pipeline(pipeline)
    }
    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        self.render_command_encoder().set_blend_constants(value)
    }
    fn set_depth_bias(&mut self, value: Option<core::DepthBias>) {
        self.render_command_encoder().set_depth_bias(value)
    }
    fn set_depth_bounds(&mut self, value: Option<core::DepthBounds>) {
        self.render_command_encoder().set_depth_bounds(value)
    }
    fn set_stencil_state(&mut self, value: &StencilState) {
        self.render_command_encoder().set_stencil_state(value)
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        self.render_command_encoder().set_viewport(value)
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        self.render_command_encoder().set_scissor_rect(value)
    }
    fn bind_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout,
        start_index: usize,
        descriptor_sets: &[DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        self.render_command_encoder().bind_descriptor_sets(
            pipeline_layout,
            start_index,
            descriptor_sets,
            dynamic_offsets,
        )
    }

    fn bind_vertex_buffers(&mut self, start_index: usize, buffers: &[(&Buffer, usize)]) {
        self.render_command_encoder().bind_vertex_buffers(start_index, buffers)
    }

    fn bind_index_buffer(&mut self, buffer: &Buffer, offset: usize, format: core::IndexFormat) {
        self.render_command_encoder().bind_index_buffer(buffer, offset, format)
    }

    fn draw(
        &mut self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        start_instance_index: u32,
    ) {
        self.render_command_encoder().draw(
            num_vertices,
            num_instances,
            start_vertex_index,
            start_instance_index,
        )
    }
    fn draw_indexed(
        &mut self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        index_offset: u32,
        start_instance_index: u32,
    ) {
        self.render_command_encoder().draw_indexed(
            num_vertices,
            num_instances,
            start_vertex_index,
            index_offset,
            start_instance_index,
        )
    }
}