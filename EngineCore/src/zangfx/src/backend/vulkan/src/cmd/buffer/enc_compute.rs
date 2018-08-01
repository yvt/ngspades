//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::version::*;
use ash::vk;

use crate::buffer::Buffer;
use crate::pipeline::ComputePipeline;
use zangfx_base as base;

use super::CmdBufferData;

impl base::ComputeCmdEncoder for CmdBufferData {
    fn bind_pipeline(&mut self, pipeline: &base::ComputePipelineRef) {
        let my_pipeline: &ComputePipeline =
            pipeline.downcast_ref().expect("bad compute pipeline type");

        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_bind_pipeline(
                self.vk_cmd_buffer(),
                vk::PipelineBindPoint::Compute,
                my_pipeline.vk_pipeline(),
            );
        }

        self.desc_set_binding_table
            .bind_root_sig(my_pipeline.root_sig());

        self.ref_table.insert_compute_pipeline(my_pipeline);
    }

    fn bind_arg_table(
        &mut self,
        index: base::ArgTableIndex,
        tables: &[(&base::ArgPoolRef, &base::ArgTableRef)],
    ) {
        self.desc_set_binding_table.bind_arg_table(index, tables);
    }

    fn dispatch(&mut self, workgroup_count: &[u32]) {
        let vk_cmd_buffer = self.vk_cmd_buffer();

        self.desc_set_binding_table.flush(
            &self.device,
            vk_cmd_buffer,
            vk::PipelineBindPoint::Compute,
        );

        let device = self.device.vk_device();

        unsafe {
            device.cmd_dispatch(
                vk_cmd_buffer,
                workgroup_count.get(0).cloned().unwrap_or(1),
                workgroup_count.get(1).cloned().unwrap_or(1),
                workgroup_count.get(2).cloned().unwrap_or(1),
            );
        }
    }

    fn dispatch_indirect(&mut self, buffer: &base::BufferRef, offset: base::DeviceSize) {
        let vk_cmd_buffer = self.vk_cmd_buffer();
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");

        self.desc_set_binding_table.flush(
            &self.device,
            vk_cmd_buffer,
            vk::PipelineBindPoint::Compute,
        );

        let device = self.device.vk_device();

        unsafe {
            device
                .fp_v1_0()
                .cmd_dispatch_indirect(vk_cmd_buffer, buffer.vk_buffer(), offset);
        }
    }
}
