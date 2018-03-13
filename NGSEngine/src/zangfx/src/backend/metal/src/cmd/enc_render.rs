//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;
use metal::MTLRenderCommandEncoder;
use base::{self, command, handles, heap, StageFlags};
use common::Rect2D;

use utils::{translate_render_stage, OCPtr};
use cmd::enc::{CmdBufferFenceSet, DebugCommands, UseResources};
use cmd::fence::Fence;
use cmd::barrier::Barrier;
use renderpipeline::RenderStateManager;

#[derive(Debug)]
pub struct RenderEncoder {
    metal_encoder: OCPtr<MTLRenderCommandEncoder>,
    fence_set: CmdBufferFenceSet,
    state: RenderStateManager,
}

zangfx_impl_object! { RenderEncoder:
command::CmdEncoder, command::RenderCmdEncoder, ::Debug }

unsafe impl Send for RenderEncoder {}
unsafe impl Sync for RenderEncoder {}

impl RenderEncoder {
    pub unsafe fn new(
        metal_encoder: MTLRenderCommandEncoder,
        fence_set: CmdBufferFenceSet,
        extents: [u32; 2],
    ) -> Self {
        Self {
            metal_encoder: OCPtr::new(metal_encoder).unwrap(),
            fence_set,
            state: RenderStateManager::new(metal_encoder, extents),
        }
    }

    pub(super) fn finish(self) -> CmdBufferFenceSet {
        self.metal_encoder.end_encoding();
        self.fence_set
    }
}

impl command::CmdEncoder for RenderEncoder {
    fn begin_debug_group(&mut self, label: &str) {
        self.metal_encoder.begin_debug_group(label);
    }

    fn end_debug_group(&mut self) {
        self.metal_encoder.end_debug_group();
    }

    fn debug_marker(&mut self, label: &str) {
        self.metal_encoder.debug_marker(label);
    }

    fn use_resource(&mut self, usage: command::ResourceUsage, objs: &[handles::ResourceRef]) {
        self.metal_encoder.use_gfx_resource(usage, objs);
    }

    fn use_heap(&mut self, heaps: &[&heap::Heap]) {
        self.metal_encoder.use_gfx_heap(heaps);
    }

    fn wait_fence(
        &mut self,
        fence: &handles::Fence,
        _src_stage: StageFlags,
        barrier: &handles::Barrier,
    ) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        let our_barrier: &Barrier = barrier.downcast_ref().expect("bad barrier type");

        let stages = our_barrier.metal_dst_stage();
        self.metal_encoder
            .wait_for_fence_before_stages(our_fence.metal_fence(), stages);

        self.fence_set.wait_fence(our_fence);
    }

    fn update_fence(&mut self, fence: &handles::Fence, src_stage: StageFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));

        let stages = translate_render_stage(src_stage);
        self.metal_encoder
            .update_fence_after_stages(our_fence.metal_fence(), stages);

        self.fence_set.signal_fence(our_fence);
    }

    fn barrier(&mut self, _barrier: &handles::Barrier) {
        self.metal_encoder.texture_barrier();
    }
}

impl command::RenderCmdEncoder for RenderEncoder {
    fn bind_pipeline(&mut self, pipeline: &handles::RenderPipeline) {
        self.state.bind_pipeline(pipeline);
    }

    fn set_blend_constant(&mut self, value: &[f32]) {
        self.state.set_blend_constant(value);
    }

    fn set_depth_bias(&mut self, value: Option<base::DepthBias>) {
        self.state.set_depth_bias(value);
    }

    fn set_depth_bounds(&mut self, value: Option<Range<f32>>) {
        self.state.set_depth_bounds(value);
    }

    fn set_stencil_refs(&mut self, values: &[u32]) {
        self.state.set_stencil_refs(values);
    }

    fn set_viewports(&mut self, start_viewport: base::ViewportIndex, value: &[base::Viewport]) {
        self.state.set_viewports(start_viewport, value);
    }

    fn set_scissors(&mut self, start_viewport: base::ViewportIndex, value: &[Rect2D<u32>]) {
        self.state.set_scissors(start_viewport, value);
    }

    fn bind_arg_table(&mut self, index: base::ArgTableIndex, tables: &[&handles::ArgTable]) {
        self.state.bind_arg_table(index, tables);
    }

    fn bind_vertex_buffers(
        &mut self,
        index: base::VertexBufferIndex,
        buffers: &[(&handles::Buffer, base::DeviceSize)],
    ) {
        self.state.bind_vertex_buffers(index, buffers);
    }

    fn bind_index_buffer(
        &mut self,
        buffers: &handles::Buffer,
        offset: base::DeviceSize,
        format: base::IndexFormat,
    ) {
        self.state.bind_index_buffer(buffers, offset, format);
    }

    fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        self.state.draw(vertex_range, instance_range);
    }

    fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        self.state
            .draw_indexed(index_buffer_range, vertex_offset, instance_range);
    }
}
