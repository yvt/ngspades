//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, metal, OCPtr};

use imp::{Backend, CommandBuffer, StencilState, GraphicsPipeline, PipelineLayout, Buffer,
          DescriptorSet, SecondaryCommandBuffer, GraphicsResourceBinder};

use std::ops::Range;

use super::descriptors::DescriptorSetBindingState;

#[derive(Debug)]
pub(crate) struct RenderCommandEncoder {
    metal_encoder: OCPtr<metal::MTLRenderCommandEncoder>,
    pipeline: Option<GraphicsPipeline>,
    descriptor_set_binding: DescriptorSetBindingState,
    index_binding: Option<(metal::MTLIndexType, Buffer, u64)>,
    extents: [u32; 2],
}

#[derive(Debug)]
pub(crate) enum GraphicsEncoderState {
    Inline(RenderCommandEncoder),
    SecondaryCommandBuffers(OCPtr<metal::MTLParallelRenderCommandEncoder>),
}

impl RenderCommandEncoder {
    pub fn new(
        metal_encoder: OCPtr<metal::MTLRenderCommandEncoder>,
        fb_extents: &[u32; 2],
    ) -> Self {
        Self {
            metal_encoder,
            pipeline: None,
            descriptor_set_binding: DescriptorSetBindingState::new(),
            index_binding: None,
            extents: fb_extents.clone(),
        }
    }

    pub fn end_encoding(&mut self) {
        self.metal_encoder.end_encoding();
        self.pipeline = None;
    }

    pub fn metal_command_encoder(&self) -> metal::MTLCommandEncoder {
        **self.metal_encoder
    }

    pub fn set_label(&self, label: &str) {
        self.metal_encoder.set_label(label);
    }

    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline) {
        self.pipeline = Some(pipeline.clone());
        pipeline.bind_pipeline_state(*self.metal_encoder, &self.extents);
    }

    fn expect_pipeline(&self) -> &GraphicsPipeline {
        self.pipeline.as_ref().expect("no pipeline")
    }

    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        self.metal_encoder.set_blend_color(
            value[0],
            value[1],
            value[2],
            value[3],
        );
    }
    fn set_depth_bias(&mut self, value: Option<core::DepthBias>) {
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
    fn set_depth_bounds(&mut self, _: Option<core::DepthBounds>) {
        panic!("not supported");
    }
    fn set_stencil_state(&mut self, value: &StencilState) {
        value.bind_depth_stencil_state(*self.metal_encoder);
    }
    fn set_stencil_reference(&mut self, values: [u32; 2]) {
        self.expect_pipeline().set_dynamic_stencil_reference(
            *self.metal_encoder,
            values,
        );
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        self.metal_encoder.set_viewport(metal::MTLViewport {
            originX: value.x as f64,
            originY: value.y as f64,
            width: value.width as f64,
            height: value.height as f64,
            znear: value.min_depth as f64,
            zfar: value.max_depth as f64,
        });
    }
    fn set_scissor_rect(&mut self, rect: &core::Rect2D<u32>) {
        self.expect_pipeline().set_dynamic_scissor_rect(
            *self.metal_encoder,
            rect,
            &self.extents,
        );
    }
    fn bind_graphics_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout,
        start_index: core::DescriptorSetBindingLocation,
        descriptor_sets: &[&DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        self.descriptor_set_binding.bind_descriptor_sets(
            &GraphicsResourceBinder(*self.metal_encoder),
            pipeline_layout,
            start_index,
            descriptor_sets,
            dynamic_offsets,
        );
    }

    fn bind_vertex_buffers(
        &mut self,
        start_index: core::VertexBindingLocation,
        buffers: &[(&Buffer, core::DeviceSize)],
    ) {
        self.expect_pipeline().bind_vertex_buffers(
            *self.metal_encoder,
            start_index,
            buffers,
        );
    }

    fn bind_index_buffer(
        &mut self,
        buffer: &Buffer,
        offset: core::DeviceSize,
        format: core::IndexFormat,
    ) {
        self.index_binding = Some((
            match format {
                core::IndexFormat::U16 => metal::MTLIndexType::UInt16,
                core::IndexFormat::U32 => metal::MTLIndexType::UInt32,
            },
            buffer.clone(),
            offset,
        ));
    }

    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        if vertex_range.len() == 0 {
            return;
        }
        if instance_range == (0..1) {
            // FIXME: this maybe causes instance index to be undefined?
            self.metal_encoder.draw_primitives(
                self.expect_pipeline().primitive_type(),
                vertex_range.start as u64,
                vertex_range.len() as u64,
            );
        } else if instance_range.len() > 0 {
            self.metal_encoder.draw_primitives_instanced(
                self.expect_pipeline().primitive_type(),
                vertex_range.start as u64,
                vertex_range.len() as u64,
                instance_range.len() as u64,
                instance_range.start as u64,
            );
        }
    }
    fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        if index_buffer_range.len() == 0 {
            return;
        }
        let &(index_type, ref index_buf, index_offset) = self.index_binding.as_ref().expect(
            "index buffer is not bound",
        );

        if instance_range == (0..1) && vertex_offset == 0 {
            // FIXME: this maybe causes instance index to be undefined?
            self.metal_encoder.draw_indexed_primitives(
                self.expect_pipeline().primitive_type(),
                index_buffer_range.len() as u64,
                index_type,
                index_buf.metal_buffer(),
                index_offset,
            );
        } else if instance_range.len() > 0 {
            self.metal_encoder.draw_indexed_primitives_instanced(
                self.expect_pipeline().primitive_type(),
                index_buffer_range.len() as u64,
                index_type,
                index_buf.metal_buffer(),
                index_offset,
                instance_range.len() as u64,
                vertex_offset as i64,
                instance_range.start as u64,
            );
        }
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
    fn set_stencil_reference(&mut self, values: [u32; 2]) {
        self.expect_graphics_pipeline().set_stencil_reference(
            values,
        )
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        self.expect_graphics_pipeline().set_viewport(value)
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        self.expect_graphics_pipeline().set_scissor_rect(value)
    }
    fn bind_graphics_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout,
        start_index: core::DescriptorSetBindingLocation,
        descriptor_sets: &[&DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        self.expect_graphics_pipeline()
            .bind_graphics_descriptor_sets(
                pipeline_layout,
                start_index,
                descriptor_sets,
                dynamic_offsets,
            )
    }

    fn bind_vertex_buffers(
        &mut self,
        start_index: core::VertexBindingLocation,
        buffers: &[(&Buffer, core::DeviceSize)],
    ) {
        self.expect_graphics_pipeline().bind_vertex_buffers(
            start_index,
            buffers,
        )
    }

    fn bind_index_buffer(
        &mut self,
        buffer: &Buffer,
        offset: core::DeviceSize,
        format: core::IndexFormat,
    ) {
        self.expect_graphics_pipeline().bind_index_buffer(
            buffer,
            offset,
            format,
        )
    }

    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        self.expect_graphics_pipeline().draw(
            vertex_range,
            instance_range,
        )
    }
    fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        self.expect_graphics_pipeline().draw_indexed(
            index_buffer_range,
            vertex_offset,
            instance_range,
        )
    }
}

impl core::RenderSubpassCommandEncoder<Backend> for SecondaryCommandBuffer {
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline) {
        self.render_command_encoder().bind_graphics_pipeline(
            pipeline,
        )
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
    fn set_stencil_reference(&mut self, values: [u32; 2]) {
        self.render_command_encoder().set_stencil_reference(values)
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        self.render_command_encoder().set_viewport(value)
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        self.render_command_encoder().set_scissor_rect(value)
    }
    fn bind_graphics_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout,
        start_index: core::DescriptorSetBindingLocation,
        descriptor_sets: &[&DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        self.render_command_encoder().bind_graphics_descriptor_sets(
            pipeline_layout,
            start_index,
            descriptor_sets,
            dynamic_offsets,
        )
    }

    fn bind_vertex_buffers(
        &mut self,
        start_index: core::VertexBindingLocation,
        buffers: &[(&Buffer, core::DeviceSize)],
    ) {
        self.render_command_encoder().bind_vertex_buffers(
            start_index,
            buffers,
        )
    }

    fn bind_index_buffer(
        &mut self,
        buffer: &Buffer,
        offset: core::DeviceSize,
        format: core::IndexFormat,
    ) {
        self.render_command_encoder().bind_index_buffer(
            buffer,
            offset,
            format,
        )
    }

    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        self.render_command_encoder().draw(
            vertex_range,
            instance_range,
        )
    }
    fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        self.render_command_encoder().draw_indexed(
            index_buffer_range,
            vertex_offset,
            instance_range,
        )
    }
}