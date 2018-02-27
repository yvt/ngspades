//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use metal::{MTLComputeCommandEncoder, MTLSize};
use base::{command, handles, heap, ArgTableIndex, StageFlags};

use utils::OCPtr;
use cmd::enc::CmdBufferFenceSet;
use cmd::fence::Fence;

#[derive(Debug)]
pub struct ComputeEncoder {
    metal_encoder: OCPtr<MTLComputeCommandEncoder>,
    fence_set: CmdBufferFenceSet,
}

zangfx_impl_object! { ComputeEncoder:
command::CmdEncoder, command::ComputeCmdEncoder, ::Debug }

impl ComputeEncoder {
    pub unsafe fn new(
        metal_encoder: MTLComputeCommandEncoder,
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

impl command::CmdEncoder for ComputeEncoder {
    fn use_resource(&mut self, _usage: command::ResourceUsage, _objs: &[handles::ResourceRef]) {
        unimplemented!();
    }

    fn use_heap(&mut self, _heaps: &[&heap::Heap]) {
        unimplemented!();
    }

    fn wait_fence(
        &mut self,
        fence: &handles::Fence,
        _src_stage: StageFlags,
        _dst_stage: StageFlags,
        _barrier: &handles::Barrier,
    ) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.metal_encoder.wait_for_fence(our_fence.metal_fence());
        self.fence_set.wait_fences.push(our_fence);
    }

    fn update_fence(&mut self, fence: &handles::Fence, _src_stage: StageFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.metal_encoder.update_fence(our_fence.metal_fence());
        self.fence_set.signal_fences.push(our_fence);
    }

    fn barrier(
        &mut self,
        _src_stage: StageFlags,
        _dst_stage: StageFlags,
        _barrier: &handles::Barrier,
    ) {
        // No-op: Metal's compute command encoders implicitly barrier between
        // each dispatch.
    }
}

impl command::ComputeCmdEncoder for ComputeEncoder {
    fn bind_pipeline(&mut self, _pipeline: &handles::ComputePipeline) {
        unimplemented!();
    }

    fn bind_arg_table(&mut self, _index: ArgTableIndex, _tables: &[&handles::ArgTable]) {
        unimplemented!();
    }

    fn dispatch(&mut self, workgroup_count: &[u32]) {
        self.metal_encoder.dispatch_threadgroups(
            MTLSize {
                width: workgroup_count.get(0).cloned().unwrap_or(1) as u64,
                height: workgroup_count.get(1).cloned().unwrap_or(1) as u64,
                depth: workgroup_count.get(2).cloned().unwrap_or(1) as u64,
            },
            unimplemented!(), // TODO: threads_per_threadgroup
        );
    }
}
