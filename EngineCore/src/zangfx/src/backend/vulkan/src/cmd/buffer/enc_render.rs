//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use arrayvec::ArrayVec;
use ash::version::*;
use ash::vk;
use ngsenumflags::flags;
use std::ops::Range;

use zangfx_base as base;
use zangfx_common::Rect2D;

use super::{CmdBufferData, EncodingState};

use crate::buffer::Buffer;
use crate::pipeline::RenderPipeline;
use crate::renderpass::RenderTargetTable;
use crate::utils::{clip_rect2d_u31, translate_rect2d_u32};

impl CmdBufferData {
    crate fn begin_render_pass(&mut self, rtt: &RenderTargetTable) {
        assert_eq!(self.state, EncodingState::NotRender);
        self.state = EncodingState::Render;

        unsafe {
            let vk_device = self.device.vk_device();
            vk_device.cmd_begin_render_pass(
                self.vk_cmd_buffer(),
                &rtt.render_pass_begin_info(),
                vk::SubpassContents::INLINE,
            );
        }

        let images = rtt.images();
        let layouts = rtt.render_pass().attachment_layouts();
        for (image, [initial_layout, final_layout]) in images.iter().zip(layouts) {
            self.use_image_for_pass(
                *initial_layout,
                *final_layout,
                flags![base::AccessType::{ColorRead | ColorWrite}],
                image,
            );
        }

        self.ref_table.insert_render_target_table(rtt);
    }

    crate fn end_render_pass(&mut self) {
        assert_eq!(self.state, EncodingState::Render);

        unsafe {
            let vk_device = self.device.vk_device();
            vk_device.cmd_end_render_pass(self.vk_cmd_buffer());
        }

        self.state = EncodingState::NotRender;

        // Process deferred fences
        if self.deferred_signal_fences.len() > 0 {
            // Can't drain `self.deferred_signal_fences` directly because
            // `self.cmd_update_fence` needs a reference to `self`
            use std::mem::replace;
            let mut deferred_signal_fences = replace(&mut self.deferred_signal_fences, Vec::new());

            for (fence_i, src_access) in deferred_signal_fences.drain(..) {
                let fence = self.ref_table.fences.get_by_index(fence_i).resource;
                self.cmd_update_fence(fence, src_access);
            }

            replace(&mut self.deferred_signal_fences, deferred_signal_fences);
        }
    }
}

impl base::RenderCmdEncoder for CmdBufferData {
    fn bind_pipeline(&mut self, pipeline: &base::RenderPipelineRef) {
        let my_pipeline: &RenderPipeline =
            pipeline.downcast_ref().expect("bad render pipeline type");

        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_bind_pipeline(
                self.vk_cmd_buffer(),
                vk::PipelineBindPoint::GRAPHICS,
                my_pipeline.vk_pipeline(),
            );
            my_pipeline.encode_partial_states(self.vk_cmd_buffer());
        }

        self.desc_set_binding_table
            .bind_root_sig(my_pipeline.root_sig());

        self.ref_table.insert_render_pipeline(my_pipeline);
    }

    fn set_blend_constant(&mut self, value: &[f32]) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_set_blend_constants(
                self.vk_cmd_buffer(),
                [value[0], value[1], value[2], value[3]],
            );
        }
    }

    fn set_depth_bias(&mut self, value: Option<base::DepthBias>) {
        let value = value.unwrap_or_default();
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.fp_v1_0().cmd_set_depth_bias(
                self.vk_cmd_buffer(),
                value.constant_factor,
                value.clamp,
                value.slope_factor,
            );
        }
    }

    fn set_depth_bounds(&mut self, value: Option<Range<f32>>) {
        let value = value.unwrap_or(0.0..1.0);
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device
                .fp_v1_0()
                .cmd_set_depth_bounds(self.vk_cmd_buffer(), value.start, value.end);
        }
    }

    fn set_stencil_refs(&mut self, values: &[u32]) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.fp_v1_0().cmd_set_stencil_reference(
                self.vk_cmd_buffer(),
                vk::StencilFaceFlags::FRONT,
                values[0],
            );
            vk_device.fp_v1_0().cmd_set_stencil_reference(
                self.vk_cmd_buffer(),
                vk::StencilFaceFlags::BACK,
                values[1],
            );
        }
    }

    fn set_viewports(&mut self, mut start_viewport: base::ViewportIndex, value: &[base::Viewport]) {
        let vk_device = self.device.vk_device();
        for values in value.chunks(16) {
            let viewports: ArrayVec<[_; 16]> = values
                .iter()
                .map(|vp| vk::Viewport {
                    x: vp.x,
                    y: vp.y,
                    width: vp.width,
                    height: vp.height,
                    min_depth: vp.min_depth,
                    max_depth: vp.max_depth,
                }).collect();
            unsafe {
                vk_device.fp_v1_0().cmd_set_viewport(
                    self.vk_cmd_buffer(),
                    start_viewport as u32,
                    viewports.len() as u32,
                    viewports.as_ptr(),
                );
            }
            start_viewport += viewports.len();
        }
    }

    fn set_scissors(&mut self, mut start_viewport: base::ViewportIndex, value: &[Rect2D<u32>]) {
        let vk_device = self.device.vk_device();
        for values in value.chunks(16) {
            let rects: ArrayVec<[_; 16]> = values
                .iter()
                .map(translate_rect2d_u32)
                .map(clip_rect2d_u31)
                .collect();
            unsafe {
                vk_device.fp_v1_0().cmd_set_scissor(
                    self.vk_cmd_buffer(),
                    start_viewport as u32,
                    rects.len() as u32,
                    rects.as_ptr(),
                );
            }
            start_viewport += rects.len();
        }
    }

    fn bind_arg_table(
        &mut self,
        index: base::ArgTableIndex,
        tables: &[(&base::ArgPoolRef, &base::ArgTableRef)],
    ) {
        self.desc_set_binding_table
            .bind_arg_table(&mut self.ref_table, index, tables);
    }

    fn bind_vertex_buffers(
        &mut self,
        mut index: base::VertexBufferIndex,
        buffers: &[(&base::BufferRef, base::DeviceSize)],
    ) {
        let vk_device = self.device.vk_device();

        for (buffer, _) in buffers.iter() {
            let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
            self.ref_table.insert_buffer(buffer);
        }

        for items in buffers.chunks(32) {
            let buffers: ArrayVec<[_; 32]> = items
                .iter()
                .map(|&(buffer, _)| {
                    let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
                    buffer.vk_buffer()
                }).collect();
            let offsets: ArrayVec<[_; 32]> = items.iter().map(|&(_, offset)| offset).collect();
            unsafe {
                vk_device.cmd_bind_vertex_buffers(
                    self.vk_cmd_buffer(),
                    index as u32,
                    &buffers,
                    &offsets,
                );
            }
            index += items.len();
        }
    }

    fn bind_index_buffer(
        &mut self,
        buffer: &base::BufferRef,
        offset: base::DeviceSize,
        format: base::IndexFormat,
    ) {
        let vk_device = self.device.vk_device();
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");

        self.ref_table.insert_buffer(buffer);

        unsafe {
            vk_device.cmd_bind_index_buffer(
                self.vk_cmd_buffer(),
                buffer.vk_buffer(),
                offset,
                match format {
                    base::IndexFormat::U16 => vk::IndexType::UINT16,
                    base::IndexFormat::U32 => vk::IndexType::UINT32,
                },
            )
        }
    }

    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        let vk_cmd_buffer = self.vk_cmd_buffer();

        self.desc_set_binding_table.flush(
            &self.device,
            vk_cmd_buffer,
            vk::PipelineBindPoint::GRAPHICS,
        );

        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_draw(
                vk_cmd_buffer,
                vertex_range.len() as u32,
                instance_range.len() as u32,
                vertex_range.start,
                instance_range.start,
            );
        }
    }

    fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        let vk_cmd_buffer = self.vk_cmd_buffer();

        self.desc_set_binding_table.flush(
            &self.device,
            vk_cmd_buffer,
            vk::PipelineBindPoint::GRAPHICS,
        );

        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_draw_indexed(
                vk_cmd_buffer,
                index_buffer_range.len() as u32,
                instance_range.len() as u32,
                index_buffer_range.start,
                vertex_offset as i32,
                instance_range.start,
            );
        }
    }

    fn draw_indirect(&mut self, buffer: &base::BufferRef, offset: base::DeviceSize) {
        let vk_cmd_buffer = self.vk_cmd_buffer();

        self.desc_set_binding_table.flush(
            &self.device,
            vk_cmd_buffer,
            vk::PipelineBindPoint::GRAPHICS,
        );

        let vk_device = self.device.vk_device();
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");

        self.ref_table.insert_buffer(buffer);

        unsafe {
            vk_device.cmd_draw_indirect(vk_cmd_buffer, buffer.vk_buffer(), offset, 1, 0);
        }
    }

    fn draw_indexed_indirect(&mut self, buffer: &base::BufferRef, offset: base::DeviceSize) {
        let vk_cmd_buffer = self.vk_cmd_buffer();

        self.desc_set_binding_table.flush(
            &self.device,
            vk_cmd_buffer,
            vk::PipelineBindPoint::GRAPHICS,
        );

        let vk_device = self.device.vk_device();
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");

        self.ref_table.insert_buffer(buffer);

        unsafe {
            vk_device.cmd_draw_indexed_indirect(vk_cmd_buffer, buffer.vk_buffer(), offset, 1, 0);
        }
    }
}
