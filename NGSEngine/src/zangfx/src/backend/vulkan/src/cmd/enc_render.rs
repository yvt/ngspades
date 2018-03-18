//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use ash::version::*;
use std::ops::Range;
use arrayvec::ArrayVec;

use base;
use common::Rect2D;

use device::DeviceRef;
use pipeline::RenderPipeline;
use buffer::Buffer;
use utils::translate_rect2d_u32;
use renderpass::RenderTargetTable;
use super::enc::{CommonCmdEncoder, DescSetBindingTable, FenceSet, RefTable};
use super::fence::Fence;

#[derive(Debug)]
pub(super) struct RenderEncoder {
    device: DeviceRef,
    vk_cmd_buffer: vk::CommandBuffer,
    fence_set: FenceSet,
    ref_table: RefTable,
    desc_set_binding_table: DescSetBindingTable,
    /// Deferred-signaled fences
    signal_fences: Vec<(Fence, base::StageFlags)>,
}

zangfx_impl_object! { RenderEncoder:
base::CmdEncoder, base::RenderCmdEncoder, ::Debug }

impl RenderEncoder {
    pub unsafe fn new(
        device: DeviceRef,
        vk_cmd_buffer: vk::CommandBuffer,
        fence_set: FenceSet,
        ref_table: RefTable,
        rtt: &RenderTargetTable,
    ) -> Self {
        let mut enc = Self {
            device,
            vk_cmd_buffer,
            fence_set,
            ref_table,
            desc_set_binding_table: DescSetBindingTable::new(),
            signal_fences: Vec::new(),
        };

        {
            let vk_device = enc.device.vk_device();
            vk_device.cmd_begin_render_pass(
                enc.vk_cmd_buffer,
                &rtt.render_pass_begin_info(),
                vk::SubpassContents::Inline,
            );

            enc.ref_table.insert_render_target_table(rtt);
        }

        enc
    }

    pub fn finish(mut self) -> (FenceSet, RefTable) {
        unsafe {
            let vk_device = self.device.vk_device();
            vk_device.cmd_end_render_pass(self.vk_cmd_buffer);
        }

        // Process deferred-signaled fences after ending a render pass
        use std::mem::replace;
        for (fence, src_stage) in replace(&mut self.signal_fences, Vec::new()) {
            self.common().update_fence(&fence, src_stage);
            self.fence_set.signal_fence(fence);
        }

        (self.fence_set, self.ref_table)
    }

    fn common(&self) -> CommonCmdEncoder {
        CommonCmdEncoder::new(self.device, self.vk_cmd_buffer)
    }
}

impl base::CmdEncoder for RenderEncoder {
    fn begin_debug_group(&mut self, label: &str) {
        self.common().begin_debug_group(label)
    }

    fn end_debug_group(&mut self) {
        self.common().end_debug_group()
    }

    fn debug_marker(&mut self, label: &str) {
        self.common().debug_marker(label)
    }

    fn use_resource(&mut self, _usage: base::ResourceUsage, _objs: &[base::ResourceRef]) {
        // No-op on Vulkan backend
    }

    fn use_heap(&mut self, _heaps: &[&base::Heap]) {
        // No-op on Vulkan backend
    }

    fn wait_fence(
        &mut self,
        fence: &base::Fence,
        _src_stage: base::StageFlags,
        _barrier: &base::Barrier,
    ) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        // Do not call `CommonCmdEncoder::wait_fence` here - the barrier is
        // already defined by the render pass. Inserting a fence wait command is
        // overkill.
        self.fence_set.wait_fence(our_fence);
    }

    fn update_fence(&mut self, fence: &base::Fence, src_stage: base::StageFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        // Defer the fence signaling until the render pass is done. It's not
        // allowed inside a render pass.
        self.signal_fences.push((our_fence, src_stage));
    }

    fn barrier(&mut self, barrier: &base::Barrier) {
        self.common().barrier(barrier)
    }
}

impl base::RenderCmdEncoder for RenderEncoder {
    fn bind_pipeline(&mut self, pipeline: &base::RenderPipeline) {
        let my_pipeline: &RenderPipeline =
            pipeline.downcast_ref().expect("bad render pipeline type");

        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_bind_pipeline(
                self.vk_cmd_buffer,
                vk::PipelineBindPoint::Graphics,
                my_pipeline.vk_pipeline(),
            );
            my_pipeline.encode_partial_states(self.vk_cmd_buffer);
        }

        self.desc_set_binding_table
            .bind_root_sig(my_pipeline.root_sig());

        self.ref_table.insert_render_pipeline(my_pipeline);
    }

    fn set_blend_constant(&mut self, value: &[f32]) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_set_blend_constants(
                self.vk_cmd_buffer,
                [value[0], value[1], value[2], value[3]],
            );
        }
    }

    fn set_depth_bias(&mut self, value: Option<base::DepthBias>) {
        let value = value.unwrap_or_default();
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.fp_v1_0().cmd_set_depth_bias(
                self.vk_cmd_buffer,
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
                .cmd_set_depth_bounds(self.vk_cmd_buffer, value.start, value.end);
        }
    }

    fn set_stencil_refs(&mut self, values: &[u32]) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.fp_v1_0().cmd_set_stencil_reference(
                self.vk_cmd_buffer,
                vk::STENCIL_FACE_FRONT_BIT,
                values[0],
            );
            vk_device.fp_v1_0().cmd_set_stencil_reference(
                self.vk_cmd_buffer,
                vk::STENCIL_FACE_BACK_BIT,
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
                })
                .collect();
            unsafe {
                vk_device.fp_v1_0().cmd_set_viewport(
                    self.vk_cmd_buffer,
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
            let rects: ArrayVec<[_; 16]> = values.iter().map(translate_rect2d_u32).collect();
            unsafe {
                vk_device.fp_v1_0().cmd_set_scissor(
                    self.vk_cmd_buffer,
                    start_viewport as u32,
                    rects.len() as u32,
                    rects.as_ptr(),
                );
            }
            start_viewport += rects.len();
        }
    }

    fn bind_arg_table(&mut self, index: base::ArgTableIndex, tables: &[&base::ArgTable]) {
        self.desc_set_binding_table.bind_arg_table(index, tables);
    }

    fn bind_vertex_buffers(
        &mut self,
        mut index: base::VertexBufferIndex,
        buffers: &[(&base::Buffer, base::DeviceSize)],
    ) {
        let vk_device = self.device.vk_device();
        for items in buffers.chunks(32) {
            let buffers: ArrayVec<[_; 32]> = items
                .iter()
                .map(|&(buffer, _)| {
                    let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
                    buffer.vk_buffer()
                })
                .collect();
            let offsets: ArrayVec<[_; 32]> = items.iter().map(|&(_, offset)| offset).collect();
            unsafe {
                vk_device.cmd_bind_vertex_buffers(
                    self.vk_cmd_buffer,
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
        buffer: &base::Buffer,
        offset: base::DeviceSize,
        format: base::IndexFormat,
    ) {
        let vk_device = self.device.vk_device();
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        unsafe {
            vk_device.cmd_bind_index_buffer(
                self.vk_cmd_buffer,
                buffer.vk_buffer(),
                offset,
                match format {
                    base::IndexFormat::U16 => vk::IndexType::Uint16,
                    base::IndexFormat::U32 => vk::IndexType::Uint32,
                },
            )
        }
    }

    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_draw(
                self.vk_cmd_buffer,
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
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_draw_indexed(
                self.vk_cmd_buffer,
                index_buffer_range.len() as u32,
                instance_range.len() as u32,
                index_buffer_range.start,
                vertex_offset as i32,
                instance_range.start,
            );
        }
    }

    fn draw_indirect(&mut self, buffer: &base::Buffer, offset: base::DeviceSize) {
        let vk_device = self.device.vk_device();
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        unsafe {
            vk_device.cmd_draw_indirect(self.vk_cmd_buffer, buffer.vk_buffer(), offset, 1, 0);
        }
    }

    fn draw_indexed_indirect(&mut self, buffer: &base::Buffer, offset: base::DeviceSize) {
        let vk_device = self.device.vk_device();
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        unsafe {
            vk_device.cmd_draw_indexed_indirect(
                self.vk_cmd_buffer,
                buffer.vk_buffer(),
                offset,
                1,
                0,
            );
        }
    }
}
