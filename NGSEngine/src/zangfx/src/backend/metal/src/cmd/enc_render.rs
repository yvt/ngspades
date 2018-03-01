//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use metal;
use metal::MTLRenderCommandEncoder;
use base::{command, handles, heap, Stage, StageFlags};

use utils::OCPtr;
use cmd::enc::{CmdBufferFenceSet, UseResources};
use cmd::fence::Fence;

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
        self.fence_set
    }
}

fn translate_render_stage(stage: StageFlags) -> metal::MTLRenderStages {
    let mut stages = metal::MTLRenderStages::empty();

    if stage.intersects(flags![
        Stage::{Top | IndirectDraw | VertexInput | Vertex | AllRender | All}])
    {
        stages |= metal::MTLRenderStageVertex;
    }

    if stage.intersects(flags![
        Stage::{Fragment | EarlyFragTests | LateFragTests | RenderOutput | Bottom | AllRender | All}])
    {
        stages |= metal::MTLRenderStageFragment;
    }

    stages
}

impl command::CmdEncoder for RenderEncoder {
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
        dst_stage: StageFlags,
        _barrier: &handles::Barrier,
    ) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));

        let stages = translate_render_stage(dst_stage);
        self.metal_encoder
            .wait_for_fence_before_stages(our_fence.metal_fence(), stages);

        self.fence_set.wait_fences.push(our_fence);
    }

    fn update_fence(&mut self, fence: &handles::Fence, src_stage: StageFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));

        let stages = translate_render_stage(src_stage);
        self.metal_encoder
            .update_fence_after_stages(our_fence.metal_fence(), stages);

        self.fence_set.signal_fences.push(our_fence);
    }

    fn barrier(
        &mut self,
        _src_stage: StageFlags,
        _dst_stage: StageFlags,
        _barrier: &handles::Barrier,
    ) {
        self.metal_encoder.texture_barrier();
    }
}

impl command::RenderCmdEncoder for RenderEncoder {}
