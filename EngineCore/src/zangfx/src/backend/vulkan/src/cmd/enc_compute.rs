//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use ash::version::*;

use base;
use device::DeviceRef;
use pipeline::ComputePipeline;
use buffer::Buffer;

use super::enc::{CommonCmdEncoder, DescSetBindingTable, FenceSet, RefTable};
use super::fence::Fence;

#[derive(Debug)]
pub(super) struct ComputeEncoder {
    device: DeviceRef,
    vk_cmd_buffer: vk::CommandBuffer,
    fence_set: FenceSet,
    ref_table: RefTable,
    desc_set_binding_table: DescSetBindingTable,
}

zangfx_impl_object! { ComputeEncoder:
base::CmdEncoder, base::ComputeCmdEncoder, ::Debug }

impl ComputeEncoder {
    pub unsafe fn new(
        device: DeviceRef,
        vk_cmd_buffer: vk::CommandBuffer,
        fence_set: FenceSet,
        ref_table: RefTable,
    ) -> Self {
        Self {
            device,
            vk_cmd_buffer,
            fence_set,
            ref_table,
            desc_set_binding_table: DescSetBindingTable::new(),
        }
    }

    pub fn finish(self) -> (FenceSet, RefTable) {
        (self.fence_set, self.ref_table)
    }

    fn common(&self) -> CommonCmdEncoder {
        CommonCmdEncoder::new(self.device, self.vk_cmd_buffer)
    }
}

impl base::CmdEncoder for ComputeEncoder {
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
        src_stage: base::StageFlags,
        barrier: &base::Barrier,
    ) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.common().wait_fence(&our_fence, src_stage, barrier);
        self.fence_set.wait_fence(our_fence);
    }

    fn update_fence(&mut self, fence: &base::Fence, src_stage: base::StageFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.common().update_fence(&our_fence, src_stage);
        self.fence_set.signal_fence(our_fence);
    }

    fn barrier(&mut self, barrier: &base::Barrier) {
        self.common().barrier(barrier)
    }
}

impl base::ComputeCmdEncoder for ComputeEncoder {
    fn bind_pipeline(&mut self, pipeline: &base::ComputePipeline) {
        let my_pipeline: &ComputePipeline =
            pipeline.downcast_ref().expect("bad compute pipeline type");

        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_bind_pipeline(
                self.vk_cmd_buffer,
                vk::PipelineBindPoint::Compute,
                my_pipeline.vk_pipeline(),
            );
        }

        self.desc_set_binding_table
            .bind_root_sig(my_pipeline.root_sig());

        self.ref_table.insert_compute_pipeline(my_pipeline);
    }

    fn bind_arg_table(&mut self, index: base::ArgTableIndex, tables: &[&base::ArgTable]) {
        self.desc_set_binding_table.bind_arg_table(index, tables);
    }

    fn dispatch(&mut self, workgroup_count: &[u32]) {
        self.desc_set_binding_table.flush(
            self.device,
            self.vk_cmd_buffer,
            vk::PipelineBindPoint::Compute,
        );

        let device = self.device.vk_device();

        unsafe {
            device.cmd_dispatch(
                self.vk_cmd_buffer,
                workgroup_count.get(0).cloned().unwrap_or(1),
                workgroup_count.get(1).cloned().unwrap_or(1),
                workgroup_count.get(2).cloned().unwrap_or(1),
            );
        }
    }

    fn dispatch_indirect(&mut self, buffer: &base::Buffer, offset: base::DeviceSize) {
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");

        self.desc_set_binding_table.flush(
            self.device,
            self.vk_cmd_buffer,
            vk::PipelineBindPoint::Compute,
        );

        let device = self.device.vk_device();

        unsafe {
            device
                .fp_v1_0()
                .cmd_dispatch_indirect(self.vk_cmd_buffer, buffer.vk_buffer(), offset);
        }
    }
}