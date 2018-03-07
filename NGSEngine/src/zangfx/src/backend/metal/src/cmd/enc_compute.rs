//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use metal::{MTLComputeCommandEncoder, MTLSize};
use base::{command, handles, heap, ArgTableIndex, StageFlags};

use utils::OCPtr;
use cmd::enc::{CmdBufferFenceSet, DebugCommands, UseResources};
use cmd::fence::Fence;
use arg::table::ArgTable;
use pipeline::ComputePipeline;

#[derive(Debug)]
pub struct ComputeEncoder {
    metal_encoder: OCPtr<MTLComputeCommandEncoder>,
    fence_set: CmdBufferFenceSet,
    threads_per_threadgroup: MTLSize,
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
            threads_per_threadgroup: MTLSize {
                width: 1,
                height: 1,
                depth: 1,
            },
        }
    }

    pub(super) fn finish(self) -> CmdBufferFenceSet {
        self.metal_encoder.end_encoding();
        self.fence_set
    }
}

impl command::CmdEncoder for ComputeEncoder {
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
        _barrier: &handles::Barrier,
    ) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.metal_encoder.wait_for_fence(our_fence.metal_fence());
        self.fence_set.wait_fence(our_fence);
    }

    fn update_fence(&mut self, fence: &handles::Fence, _src_stage: StageFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.metal_encoder.update_fence(our_fence.metal_fence());
        self.fence_set.signal_fence(our_fence);
    }

    fn barrier(&mut self, _barrier: &handles::Barrier) {
        // No-op: Metal's compute command encoders implicitly barrier between
        // each dispatch.
    }
}

impl command::ComputeCmdEncoder for ComputeEncoder {
    fn bind_pipeline(&mut self, pipeline: &handles::ComputePipeline) {
        let our_pipeline: &ComputePipeline =
            pipeline.downcast_ref().expect("bad compute pipeline type");
        self.metal_encoder
            .set_compute_pipeline_state(our_pipeline.metal_pipeline());
        self.threads_per_threadgroup = our_pipeline.threads_per_threadgroup();
    }

    fn bind_arg_table(&mut self, index: ArgTableIndex, tables: &[&handles::ArgTable]) {
        for (i, table) in tables.iter().enumerate() {
            let our_table: &ArgTable = table.downcast_ref().expect("bad argument table type");
            self.metal_encoder.set_buffer(
                (i + index) as u64,
                our_table.offset() as u64,
                our_table.metal_buffer(),
            );
        }
    }

    fn dispatch(&mut self, workgroup_count: &[u32]) {
        self.metal_encoder.dispatch_threadgroups(
            MTLSize {
                width: workgroup_count.get(0).cloned().unwrap_or(1) as u64,
                height: workgroup_count.get(1).cloned().unwrap_or(1) as u64,
                depth: workgroup_count.get(2).cloned().unwrap_or(1) as u64,
            },
            self.threads_per_threadgroup,
        );
    }
}
