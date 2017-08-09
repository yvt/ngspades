//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;
use std::ops::Range;
use smallvec::SmallVec;

use imp::{CommandBuffer, SecondaryCommandBuffer, GraphicsPipeline, PipelineLayout, DescriptorSet,
          StencilState, Buffer};
use {DeviceRef, Backend, AshDevice};

struct GraphicsEncoder<'a>(&'a AshDevice, vk::CommandBuffer);

impl<'a> GraphicsEncoder<'a> {
    // TODO: Do not allow draw calls until all descriptor set bindings are set properly
    fn bind_graphics_pipeline<T: DeviceRef>(&self, pipeline: &GraphicsPipeline<T>) {
        unsafe {
            self.0.cmd_bind_pipeline(
                self.1,
                vk::PipelineBindPoint::Graphics,
                pipeline.handle(),
            )
        };
    }
    fn set_blend_constants(&self, _: &[f32; 4]) {
        // it exists in the Vulkan spec but `DeviceV1_0` does not implement it
        unimplemented!()
    }
    fn set_depth_bias(&self, _: Option<core::DepthBias>) {
        // it exists in the Vulkan spec but `DeviceV1_0` does not implement it
        unimplemented!()
    }
    fn set_depth_bounds(&self, _: Option<core::DepthBounds>) {
        // it exists in the Vulkan spec but `DeviceV1_0` does not implement it
        unimplemented!()
    }
    fn set_stencil_state(&self, _: &StencilState) {
        // it exists in the Vulkan spec but `DeviceV1_0` does not implement it
        unimplemented!()
    }
    fn set_stencil_reference(&self, _: [u32; 2]) {
        // it exists in the Vulkan spec but `DeviceV1_0` does not implement it
        unimplemented!()
    }
    fn set_viewport(&self, value: &core::Viewport) {
        unsafe {
            self.0.cmd_set_viewport(
                self.1,
                &[
                    vk::Viewport {
                        x: value.x,
                        y: value.y,
                        width: value.width,
                        height: value.height,
                        min_depth: value.min_depth,
                        max_depth: value.max_depth,
                    },
                ],
            );
        }
    }
    fn set_scissor_rect(&self, value: &core::Rect2D<u32>) {
        unsafe {
            self.0.cmd_set_scissor(
                self.1,
                &[
                    vk::Rect2D {
                        offset: vk::Offset2D {
                            x: value.min.x as i32,
                            y: value.min.y as i32,
                        },
                        extent: vk::Extent2D {
                            width: value.max.x.saturating_sub(value.min.x),
                            height: value.max.y.saturating_sub(value.min.y),
                        },
                    },
                ],
            );
        }
    }
    fn bind_graphics_descriptor_sets<T: DeviceRef>(
        &self,
        pipeline_layout: &PipelineLayout<T>,
        start_index: core::DescriptorSetBindingLocation,
        descriptor_sets: &[&DescriptorSet<T>],
        dynamic_offsets: &[u32],
    ) {
        let desc_sets: SmallVec<[_; 8]> = descriptor_sets.iter().map(|ds| ds.handle()).collect();
        unsafe {
            self.0.cmd_bind_descriptor_sets(
                self.1,
                vk::PipelineBindPoint::Graphics,
                pipeline_layout.handle(),
                start_index as u32,
                &desc_sets,
                dynamic_offsets,
            );
        }
    }

    fn bind_vertex_buffers<T: DeviceRef>(
        &self,
        start_index: core::VertexBindingLocation,
        buffer_offsets: &[(&Buffer<T>, core::DeviceSize)],
    ) {
        let buffers: SmallVec<[_; 32]> = buffer_offsets.iter().map(|x| x.0.handle()).collect();
        let offsets: SmallVec<[_; 32]> = buffer_offsets.iter().map(|x| x.1).collect();
        unsafe {
            self.0.cmd_bind_vertex_buffers(
                self.1,
                start_index as u32,
                &buffers,
                &offsets,
            );
        }
    }

    fn bind_index_buffer<T: DeviceRef>(
        &self,
        buffer: &Buffer<T>,
        offset: core::DeviceSize,
        format: core::IndexFormat,
    ) {
        let index_type = match format {
            core::IndexFormat::U16 => vk::IndexType::Uint16,
            core::IndexFormat::U32 => vk::IndexType::Uint32,
        };
        unsafe {
            self.0.cmd_bind_index_buffer(
                self.1,
                buffer.handle(),
                offset,
                index_type,
            );
        }
    }

    fn draw(&self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        unsafe {
            self.0.cmd_draw(
                self.1,
                vertex_range.len() as u32,
                instance_range.len() as u32,
                vertex_range.start,
                instance_range.start,
            );
        }
    }
    fn draw_indexed(
        &self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        unsafe {
            self.0.cmd_draw_indexed(
                self.1,
                index_buffer_range.len() as u32,
                instance_range.len() as u32,
                index_buffer_range.start,
                vertex_offset as i32,
                instance_range.start,
            );
        }
    }
}

impl<T: DeviceRef> CommandBuffer<T> {
    fn graphics_encoder(&mut self) -> Option<GraphicsEncoder> {
        if self.encoder_error().is_some() {
            None
        } else {
            Some(GraphicsEncoder(
                &self.data.device_ref.device(),
                self.expect_render_subpass_inline().buffer,
            ))
        }
    }
}

impl<T: DeviceRef> core::RenderSubpassCommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline<T>) {
        if let Some(table) = self.dependency_table() {
            table.insert_graphics_pipeline(pipeline);
        }
        if let Some(e) = self.graphics_encoder() {
            e.bind_graphics_pipeline(pipeline);
        }
    }
    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        if let Some(e) = self.graphics_encoder() {
            e.set_blend_constants(value);
        }
    }
    fn set_depth_bias(&mut self, value: Option<core::DepthBias>) {
        if let Some(e) = self.graphics_encoder() {
            e.set_depth_bias(value);
        }
    }
    fn set_depth_bounds(&mut self, value: Option<core::DepthBounds>) {
        if let Some(e) = self.graphics_encoder() {
            e.set_depth_bounds(value);
        }
    }
    fn set_stencil_state(&mut self, value: &StencilState) {
        if let Some(e) = self.graphics_encoder() {
            e.set_stencil_state(value);
        }
    }
    fn set_stencil_reference(&mut self, values: [u32; 2]) {
        if let Some(e) = self.graphics_encoder() {
            e.set_stencil_reference(values);
        }
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        if let Some(e) = self.graphics_encoder() {
            e.set_viewport(value);
        }
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        if let Some(e) = self.graphics_encoder() {
            e.set_scissor_rect(value);
        }
    }
    fn bind_graphics_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout<T>,
        start_index: core::DescriptorSetBindingLocation,
        descriptor_sets: &[&DescriptorSet<T>],
        dynamic_offsets: &[u32],
    ) {
        if let Some(table) = self.dependency_table() {
            table.insert_pipeline_layout(pipeline_layout);
            for ds in descriptor_sets.iter() {
                table.insert_descriptor_set(ds);
            }
        }
        if let Some(e) = self.graphics_encoder() {
            e.bind_graphics_descriptor_sets(
                pipeline_layout,
                start_index,
                descriptor_sets,
                dynamic_offsets,
            );
        }
    }

    fn bind_vertex_buffers(
        &mut self,
        start_index: core::VertexBindingLocation,
        buffers: &[(&Buffer<T>, core::DeviceSize)],
    ) {
        if let Some(table) = self.dependency_table() {
            for buffer in buffers.iter() {
                table.insert_buffer(buffer.0);
            }
        }
        if let Some(e) = self.graphics_encoder() {
            e.bind_vertex_buffers(start_index, buffers);
        }
    }

    fn bind_index_buffer(
        &mut self,
        buffer: &Buffer<T>,
        offset: core::DeviceSize,
        format: core::IndexFormat,
    ) {
        if let Some(table) = self.dependency_table() {
            table.insert_buffer(buffer);
        }
        if let Some(e) = self.graphics_encoder() {
            e.bind_index_buffer(buffer, offset, format);
        }
    }

    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        if let Some(e) = self.graphics_encoder() {
            e.draw(vertex_range, instance_range);
        }
    }
    fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        if let Some(e) = self.graphics_encoder() {
            e.draw_indexed(index_buffer_range, vertex_offset, instance_range);
        }
    }
}

impl<T: DeviceRef> SecondaryCommandBuffer<T> {
    fn graphics_encoder(&mut self) -> Option<GraphicsEncoder> {
        self.expect_active().map(|scbd| {
            GraphicsEncoder(scbd.device_ref.device(), scbd.buffer)
        })
    }
}

impl<T: DeviceRef> core::RenderSubpassCommandEncoder<Backend<T>> for SecondaryCommandBuffer<T> {
    // these are *exact* copies of those of `CommandBuffer<T>`
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline<T>) {
        if let Some(table) = self.dependency_table() {
            table.insert_graphics_pipeline(pipeline);
        }
        if let Some(e) = self.graphics_encoder() {
            e.bind_graphics_pipeline(pipeline);
        }
    }
    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        if let Some(e) = self.graphics_encoder() {
            e.set_blend_constants(value);
        }
    }
    fn set_depth_bias(&mut self, value: Option<core::DepthBias>) {
        if let Some(e) = self.graphics_encoder() {
            e.set_depth_bias(value);
        }
    }
    fn set_depth_bounds(&mut self, value: Option<core::DepthBounds>) {
        if let Some(e) = self.graphics_encoder() {
            e.set_depth_bounds(value);
        }
    }
    fn set_stencil_state(&mut self, value: &StencilState) {
        if let Some(e) = self.graphics_encoder() {
            e.set_stencil_state(value);
        }
    }
    fn set_stencil_reference(&mut self, values: [u32; 2]) {
        if let Some(e) = self.graphics_encoder() {
            e.set_stencil_reference(values);
        }
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        if let Some(e) = self.graphics_encoder() {
            e.set_viewport(value);
        }
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        if let Some(e) = self.graphics_encoder() {
            e.set_scissor_rect(value);
        }
    }
    fn bind_graphics_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout<T>,
        start_index: core::DescriptorSetBindingLocation,
        descriptor_sets: &[&DescriptorSet<T>],
        dynamic_offsets: &[u32],
    ) {
        if let Some(table) = self.dependency_table() {
            table.insert_pipeline_layout(pipeline_layout);
            for ds in descriptor_sets.iter() {
                table.insert_descriptor_set(ds);
            }
        }
        if let Some(e) = self.graphics_encoder() {
            e.bind_graphics_descriptor_sets(
                pipeline_layout,
                start_index,
                descriptor_sets,
                dynamic_offsets,
            );
        }
    }

    fn bind_vertex_buffers(
        &mut self,
        start_index: core::VertexBindingLocation,
        buffers: &[(&Buffer<T>, core::DeviceSize)],
    ) {
        if let Some(table) = self.dependency_table() {
            for buffer in buffers.iter() {
                table.insert_buffer(buffer.0);
            }
        }
        if let Some(e) = self.graphics_encoder() {
            e.bind_vertex_buffers(start_index, buffers);
        }
    }

    fn bind_index_buffer(
        &mut self,
        buffer: &Buffer<T>,
        offset: core::DeviceSize,
        format: core::IndexFormat,
    ) {
        if let Some(table) = self.dependency_table() {
            table.insert_buffer(buffer);
        }
        if let Some(e) = self.graphics_encoder() {
            e.bind_index_buffer(buffer, offset, format);
        }
    }

    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        if let Some(e) = self.graphics_encoder() {
            e.draw(vertex_range, instance_range);
        }
    }
    fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        if let Some(e) = self.graphics_encoder() {
            e.draw_indexed(index_buffer_range, vertex_offset, instance_range);
        }
    }
}
