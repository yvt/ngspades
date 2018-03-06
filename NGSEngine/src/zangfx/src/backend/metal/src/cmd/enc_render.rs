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

#[derive(Debug)]
pub struct RenderEncoder {
    metal_encoder: OCPtr<MTLRenderCommandEncoder>,
    fence_set: CmdBufferFenceSet,
}

zangfx_impl_object! { RenderEncoder:
command::CmdEncoder, command::RenderCmdEncoder, ::Debug }

impl RenderEncoder {
    pub unsafe fn new(
        metal_encoder: MTLRenderCommandEncoder,
        fence_set: CmdBufferFenceSet,
    ) -> Self {
        Self {
            metal_encoder: OCPtr::new(metal_encoder).unwrap(),
            fence_set,
        }
    }

    pub fn finish(self) -> CmdBufferFenceSet {
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
    fn bind_pipeline(&mut self, _pipeline: &handles::RenderPipeline) {
        unimplemented!()
    }

    fn set_blend_constant(&mut self, _value: &[f32]) {
        unimplemented!()
    }

    fn set_depth_bias(&mut self, _value: Option<base::DepthBias>) {
        unimplemented!()
    }

    fn set_depth_bounds(&mut self, _value: Option<Range<f32>>) {
        unimplemented!()
    }

    fn set_stencil_state(&mut self, _value: &[base::StencilMasks]) {
        unimplemented!()
    }

    fn set_stencil_refs(&mut self, _values: &[u32]) {
        unimplemented!()
    }

    fn set_viewports(&mut self, _start_viewport: base::ViewportIndex, _value: &[base::Viewport]) {
        unimplemented!()
    }

    fn set_scissors(&mut self, _start_viewport: base::ViewportIndex, _value: &[Rect2D<u32>]) {
        unimplemented!()
    }

    fn bind_arg_table(&mut self, _index: base::ArgTableIndex, _tables: &[&handles::ArgTable]) {
        unimplemented!()
    }

    fn bind_vertex_buffers(
        &mut self,
        _index: base::VertexBufferIndex,
        _buffers: &[(&handles::Buffer, base::DeviceSize)],
    ) {
        unimplemented!()
    }

    fn bind_index_buffer(
        &mut self,
        _buffers: &handles::Buffer,
        _offset: base::DeviceSize,
        _format: base::IndexFormat,
    ) {
        unimplemented!()
    }

    fn draw(&mut self, _vertex_range: Range<u32>, _instance_range: Range<u32>) {
        unimplemented!()
    }

    fn draw_indexed(
        &mut self,
        _index_buffer_range: Range<u32>,
        _vertex_offset: u32,
        _instance_range: Range<u32>,
    ) {
        unimplemented!()
    }
}
