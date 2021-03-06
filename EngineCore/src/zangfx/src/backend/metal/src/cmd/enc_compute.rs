//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use zangfx_base::{self as base, command, heap, zangfx_impl_object, ArgTableIndex, DeviceSize};
use zangfx_metal_rs::{MTLComputeCommandEncoder, MTLSize};

use crate::arg::table::ArgTable;
use crate::buffer::Buffer;
use crate::cmd::enc::{CmdBufferFenceSet, DebugCommands, UseResources};
use crate::cmd::fence::Fence;
use crate::computepipeline::ComputePipeline;
use crate::utils::OCPtr;

#[derive(Debug)]
crate struct ComputeEncoder {
    metal_encoder: OCPtr<MTLComputeCommandEncoder>,
    fence_set: CmdBufferFenceSet,
    threads_per_threadgroup: MTLSize,
}

zangfx_impl_object! { ComputeEncoder:
dyn command::CmdEncoder, dyn command::ComputeCmdEncoder, dyn crate::Debug }

unsafe impl Send for ComputeEncoder {}
unsafe impl Sync for ComputeEncoder {}

impl ComputeEncoder {
    crate unsafe fn new(
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

    fn use_resource_core(&mut self, usage: base::ResourceUsageFlags, objs: base::ResourceSet<'_>) {
        self.metal_encoder.use_gfx_resource(usage, objs);
    }

    fn use_heap(&mut self, heaps: &[&heap::HeapRef]) {
        self.metal_encoder.use_gfx_heap(heaps);
    }

    fn wait_fence(&mut self, fence: &base::FenceRef, _dst_access: base::AccessTypeFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.metal_encoder.wait_for_fence(our_fence.metal_fence());
        self.fence_set.wait_fence(our_fence);
    }

    fn update_fence(&mut self, fence: &base::FenceRef, _src_access: base::AccessTypeFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.metal_encoder.update_fence(our_fence.metal_fence());
        self.fence_set.signal_fence(our_fence);
    }

    fn barrier_core(
        &mut self,
        _obj: base::ResourceSet<'_>,
        _src_access: base::AccessTypeFlags,
        _dst_access: base::AccessTypeFlags,
    ) {
        // No-op: Metal's compute command encoders implicitly barrier between
        // each dispatch.
    }
}

impl command::ComputeCmdEncoder for ComputeEncoder {
    fn bind_pipeline(&mut self, pipeline: &base::ComputePipelineRef) {
        let our_pipeline: &ComputePipeline =
            pipeline.downcast_ref().expect("bad compute pipeline type");
        self.metal_encoder
            .set_compute_pipeline_state(our_pipeline.metal_pipeline());
        self.threads_per_threadgroup = our_pipeline.threads_per_threadgroup();
    }

    fn bind_arg_table(
        &mut self,
        index: ArgTableIndex,
        tables: &[(&base::ArgPoolRef, &base::ArgTableRef)],
    ) {
        for (i, (pool, table)) in tables.iter().enumerate() {
            let our_table: &ArgTable = table.downcast_ref().expect("bad argument table type");
            self.metal_encoder.set_buffer(
                (i + index) as u64,
                our_table.offset() as u64,
                our_table.metal_buffer(pool),
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

    fn dispatch_indirect(&mut self, buffer: &base::BufferRef, offset: DeviceSize) {
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        let (metal_buffer, buffer_offset) = buffer.metal_buffer_and_offset().unwrap();
        self.metal_encoder
            .dispatch_threadgroups_with_indirect_buffer(
                metal_buffer,
                offset + buffer_offset,
                self.threads_per_threadgroup,
            );
    }
}
