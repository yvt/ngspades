//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use metal::MTLRenderCommandEncoder;
use base::{command, handles, heap, StageFlags};

use utils::{translate_render_stage, OCPtr};
use cmd::enc::{CmdBufferFenceSet, UseResources};
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

impl command::RenderCmdEncoder for RenderEncoder {}
