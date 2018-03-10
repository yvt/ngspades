//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use ash::version::*;
use std::collections::HashSet;

use base;

use cmd::fence::Fence;
use device::DeviceRef;
use arg::layout::RootSig;
use arg::pool::ArgTable;
use pipeline::ComputePipeline;

#[derive(Debug, Default)]
pub struct FenceSet {
    pub wait_fences: Vec<Fence>,
    pub signal_fences: HashSet<Fence>,
}

impl FenceSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn wait_fence(&mut self, fence: Fence) {
        if self.signal_fences.contains(&fence) {
            // Found a matching fence signaling operating in the same CB
            return;
        }
        self.wait_fences.push(fence);
    }

    pub fn signal_fence(&mut self, fence: Fence) {
        self.signal_fences.insert(fence);
    }
}

/// Objects associated with a command buffer. This type is used for the
/// following two purposes:
///
///  1. To pass objects with a command buffer to the queue scheduler.
///  2. To retain references to the objects until the exection of the command
///     buffer is done.
///
#[derive(Debug, Default)]
pub struct RefTable {
    compute_pipelines: HashSet<ComputePipeline>,
}

impl RefTable {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert_compute_pipeline(&mut self, obj: &ComputePipeline) {
        self.compute_pipelines.insert(obj.clone());
    }
}

#[derive(Debug)]
pub(super) struct CommonCmdEncoder {
    device: DeviceRef,
    vk_cmd_buffer: vk::CommandBuffer,
}

impl CommonCmdEncoder {
    pub fn new(device: DeviceRef, vk_cmd_buffer: vk::CommandBuffer) -> Self {
        Self {
            device,
            vk_cmd_buffer,
        }
    }

    pub fn begin_debug_group(&mut self, _label: &str) {
        // TODO: debug commands
    }

    pub fn end_debug_group(&mut self) {
        // TODO: debug commands
    }

    pub fn debug_marker(&mut self, _label: &str) {
        // TODO: debug commands
    }

    pub fn wait_fence(
        &mut self,
        _fence: &Fence,
        _src_stage: base::StageFlags,
        _barrier: &base::Barrier,
    ) {
        // TODO
    }

    pub fn update_fence(&mut self, _fence: &Fence, _src_stage: base::StageFlags) {
        // TODO
    }

    pub fn barrier(&mut self, _barrier: &base::Barrier) {
        // TODO
    }
}

#[derive(Debug)]
pub(super) struct DescSetBindingTable {
    /// The first arugment table index that needs rebinding.
    start_dirty: usize,
    table_sig_id: [usize; ::MAX_NUM_ARG_TABLES],
    desc_sets: [vk::DescriptorSet; ::MAX_NUM_ARG_TABLES],

    /// The root signature of the currently bound pipeline.
    bound_root_sig: Option<RootSig>,
}

impl DescSetBindingTable {
    pub fn new() -> Self {
        Self {
            start_dirty: 0,
            table_sig_id: [0; ::MAX_NUM_ARG_TABLES],
            desc_sets: [vk::DescriptorSet::null(); ::MAX_NUM_ARG_TABLES],
            bound_root_sig: None,
        }
    }

    pub fn bind_root_sig(&mut self, root_sig: &RootSig) {
        self.bound_root_sig = Some(root_sig.clone());
    }

    pub fn bind_arg_table(&mut self, index: base::ArgTableIndex, tables: &[&base::ArgTable]) {
        use std::cmp::min;

        if tables.len() == 0 {
            return;
        }

        for (i, table) in tables.iter().enumerate() {
            let my_table: &ArgTable = table.downcast_ref().expect("bad argument table type");
            self.desc_sets[i + index] = my_table.vk_descriptor_set();
        }

        self.start_dirty = min(self.start_dirty, index);
    }

    pub fn flush(
        &mut self,
        device: DeviceRef,
        vk_cmd_buffer: vk::CommandBuffer,
        bind_point: vk::PipelineBindPoint,
    ) {
        use std::cmp::min;

        let root_sig = self.bound_root_sig.as_ref().expect("no bound pipeline");
        let table_sigs = root_sig.tables();

        // Compare the pipeline layout against the last one, and mark the
        // incompatible part as dirty.
        self.start_dirty = min(self.start_dirty, table_sigs.len());
        for i in (0..self.start_dirty).rev() {
            if table_sigs[i].id() != self.table_sig_id[i] {
                self.start_dirty = i;
                self.table_sig_id[i] = table_sigs[i].id();
            }
        }

        // Emit bind commands
        if self.start_dirty < table_sigs.len() {
            let vk_device = device.vk_device();
            unsafe {
                vk_device.cmd_bind_descriptor_sets(
                    vk_cmd_buffer,
                    bind_point,
                    root_sig.vk_pipeline_layout(),
                    self.start_dirty as u32,
                    &self.desc_sets[self.start_dirty..table_sigs.len()],
                    &[],
                );
            }
        }

        self.start_dirty = table_sigs.len();
    }
}
