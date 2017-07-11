//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use std::ops::Range;

use imp::{CommandBuffer, SecondaryCommandBuffer, GraphicsPipeline, PipelineLayout, DescriptorSet,
          StencilState, Buffer};
use {DeviceRef, Backend};

impl<T: DeviceRef> core::RenderSubpassCommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline<T>) {
        unimplemented!()
    }
    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        unimplemented!()
    }
    fn set_depth_bias(&mut self, value: Option<core::DepthBias>) {
        unimplemented!()
    }
    fn set_depth_bounds(&mut self, value: Option<core::DepthBounds>) {
        unimplemented!()
    }
    fn set_stencil_state(&mut self, value: &StencilState<T>) {
        unimplemented!()
    }
    fn set_stencil_reference(&mut self, values: [u32; 2]) {
        unimplemented!()
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        unimplemented!()
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        unimplemented!()
    }
    fn bind_graphics_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout<T>,
        start_index: core::DescriptorSetBindingLocation,
        descriptor_sets: &[&DescriptorSet<T>],
        dynamic_offsets: &[u32],
    ) {
        unimplemented!()
    }

    fn bind_vertex_buffers(
        &mut self,
        start_index: core::VertexBindingLocation,
        buffers: &[(&Buffer<T>, core::DeviceSize)],
    ) {
        unimplemented!()
    }

    fn bind_index_buffer(
        &mut self,
        buffer: &Buffer<T>,
        offset: core::DeviceSize,
        format: core::IndexFormat,
    ) {
        unimplemented!()
    }

    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        unimplemented!()
    }
    fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        unimplemented!()
    }
}

impl<T: DeviceRef> core::RenderSubpassCommandEncoder<Backend<T>> for SecondaryCommandBuffer<T> {
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline<T>) {
        unimplemented!()
    }
    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        unimplemented!()
    }
    fn set_depth_bias(&mut self, value: Option<core::DepthBias>) {
        unimplemented!()
    }
    fn set_depth_bounds(&mut self, value: Option<core::DepthBounds>) {
        unimplemented!()
    }
    fn set_stencil_state(&mut self, value: &StencilState<T>) {
        unimplemented!()
    }
    fn set_stencil_reference(&mut self, values: [u32; 2]) {
        unimplemented!()
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        unimplemented!()
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        unimplemented!()
    }
    fn bind_graphics_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout<T>,
        start_index: core::DescriptorSetBindingLocation,
        descriptor_sets: &[&DescriptorSet<T>],
        dynamic_offsets: &[u32],
    ) {
        unimplemented!()
    }

    fn bind_vertex_buffers(
        &mut self,
        start_index: core::VertexBindingLocation,
        buffers: &[(&Buffer<T>, core::DeviceSize)],
    ) {
        unimplemented!()
    }

    fn bind_index_buffer(
        &mut self,
        buffer: &Buffer<T>,
        offset: core::DeviceSize,
        format: core::IndexFormat,
    ) {
        unimplemented!()
    }

    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        unimplemented!()
    }
    fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        unimplemented!()
    }
}
