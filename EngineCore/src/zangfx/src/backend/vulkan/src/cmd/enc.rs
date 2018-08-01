//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::version::*;
use ash::vk;
use std::collections::HashSet;

use zangfx_base as base;

use crate::arg::layout::RootSig;
use crate::arg::pool::ArgTable;
use crate::cmd::fence::Fence;
use crate::device::DeviceRef;
use crate::pipeline::{ComputePipeline, RenderPipeline};
use crate::renderpass::RenderTargetTable;
use crate::resstate::{CmdBuffer, RefTable};

#[derive(Debug, Default)]
crate struct FenceSet {
    /// A set of fence that must be signaled before executing the command
    /// buffer.
    ///
    /// Each entry is an index into `RefTableSet::fences`.
    crate wait_fences: Vec<usize>,

    /// A set of fence that will be signaled by the command buffer.
    ///
    /// Each entry is an index into `RefTableSet::fences`.
    crate signal_fences: Vec<usize>,
}

impl FenceSet {
    crate fn new() -> Self {
        Default::default()
    }

    crate fn wait_fence(&mut self, ref_table_set: &mut RefTableSet, fence: &Fence) {
        let ref_entry = ref_table_set
            .fences
            .get_mut(&mut ref_table_set.cmd_buffer, fence);

        if ref_entry.op.signaled {
            // Found a matching fence signaling operation in the same CB
            return;
        }

        self.wait_fences.push(ref_entry.index);
    }

    crate fn signal_fence(&mut self, ref_table_set: &mut RefTableSet, fence: &Fence) {
        let ref_entry = ref_table_set
            .fences
            .get_mut(&mut ref_table_set.cmd_buffer, fence);

        self.signal_fences.push(ref_entry.index);
    }
}

/// Objects associated with a command buffer. This type is used for the
/// following two purposes:
///
///  1. To pass objects with a command buffer to the queue scheduler.
///  2. To retain references to the objects until the exection of the command
///     buffer is done.
///  3. Resource state tracking.
///
#[derive(Debug)]
crate struct RefTableSet {
    /// The access token used to access the per-command buffer resource states.
    cmd_buffer: CmdBuffer,

    compute_pipelines: HashSet<ComputePipeline>,
    render_pipelines: HashSet<RenderPipeline>,
    render_target_tables: HashSet<RenderTargetTable>,

    crate fences: RefTable<Fence, FenceOp>,
}

/// The locally tracked state of a fence for a command buffer.
#[derive(Debug, Default)]
crate struct FenceOp {
    /// If this is `true`, this fence is signaled by one of the commands
    /// previously encoded to the same `CmdBuffer`.
    ///
    /// `true` iff this fence is in `FenceSet::signal_fences`.
    crate signaled: bool,
}

impl RefTableSet {
    crate fn new(resstate_cb: CmdBuffer) -> Self {
        Self {
            cmd_buffer: resstate_cb,
            fences: Default::default(),
            compute_pipelines: Default::default(),
            render_pipelines: Default::default(),
            render_target_tables: Default::default(),
        }
    }

    crate fn clear(&mut self) {
        self.fences.clear(&mut self.cmd_buffer, |_, _| {});
        self.compute_pipelines.clear();
        self.render_pipelines.clear();
        self.render_target_tables.clear();
    }

    crate fn insert_compute_pipeline(&mut self, obj: &ComputePipeline) {
        self.compute_pipelines.insert(obj.clone());
    }

    crate fn insert_render_pipeline(&mut self, obj: &RenderPipeline) {
        self.render_pipelines.insert(obj.clone());
    }

    crate fn insert_render_target_table(&mut self, obj: &RenderTargetTable) {
        self.render_target_tables.insert(obj.clone());
    }
}

#[derive(Debug)]
pub(super) struct CommonCmdEncoder {
    device: DeviceRef,
    vk_cmd_buffer: vk::CommandBuffer,
}

impl CommonCmdEncoder {
    crate fn new(device: DeviceRef, vk_cmd_buffer: vk::CommandBuffer) -> Self {
        Self {
            device,
            vk_cmd_buffer,
        }
    }

    crate fn begin_debug_group(&mut self, _label: &str) {
        // TODO: debug commands
    }

    crate fn end_debug_group(&mut self) {
        // TODO: debug commands
    }

    crate fn debug_marker(&mut self, _label: &str) {
        // TODO: debug commands
    }

    /// Implementation of `wait_fence` used by all command encoders except
    /// the render encoder.
    ///
    /// A render pass automatically inserts memory barriers as defined by
    /// external subpass dependencies, and ZanGFX requires that they must be
    /// a conservative approximation of the barrier inserted by fences.
    crate fn wait_fence(&mut self, _fence: &Fence, _dst_access: base::AccessTypeFlags) {
        unimplemented!()
        /*
        let traits = self.device.caps().info.traits;
        if traits.intersects(DeviceTrait::MoltenVK) {
            // Skip all event operations on MoltenVK
            return;
        }

        let my_barrier: &Barrier = barrier.downcast_ref().expect("bad barrier type");
        let data = my_barrier.data();
        debug_assert_eq!(
            data.src_stage_mask & translate_pipeline_stage_flags(src_stage),
            data.src_stage_mask,
            "Valid usage violation: \
             The supported stages of the first access type of each barrier \
             defined by `barrier` must be a subset of `src_stage`."
        );

        if data.dst_stage_mask.is_empty() {
            return;
        }

        let device = self.device.vk_device();
        unsafe {
            device.fp_v1_0().cmd_wait_events(
                self.vk_cmd_buffer,
                1,
                &fence.vk_event(),
                if src_stage.is_empty() {
                    vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT
                } else {
                    translate_pipeline_stage_flags(src_stage)
                },
                if data.dst_stage_mask.is_empty() {
                    vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT
                } else {
                    data.dst_stage_mask
                },
                data.global_barriers.len() as u32,
                data.global_barriers.as_ptr(),
                data.buffer_barriers.len() as u32,
                data.buffer_barriers.as_ptr(),
                data.image_barriers.len() as u32,
                data.image_barriers.as_ptr(),
            );
        }*/
    }

    /// Implementation of `update_fence` used by all command encoders.
    ///
    /// When calling this from a render encoder, this must be called after
    /// ending a render pass.
    crate fn update_fence(&mut self, _fence: &Fence, _src_access: base::AccessTypeFlags) {
        unimplemented!()
        /*let traits = self.device.caps().info.traits;
        if traits.intersects(DeviceTrait::MoltenVK) {
            // Skip all event operations on MoltenVK
            return;
        }

        let device = self.device.vk_device();
        unsafe {
            device.fp_v1_0().cmd_set_event(
                self.vk_cmd_buffer,
                fence.vk_event(),
                if src_stage.is_empty() {
                    vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT
                } else {
                    translate_pipeline_stage_flags(src_stage)
                },
            );
        }*/
    }

    crate fn barrier_core(
        &mut self,
        _obj: base::ResourceSet<'_>,
        _src_access: base::AccessTypeFlags,
        _dst_access: base::AccessTypeFlags,
    ) {
        unimplemented!()
        /* let my_barrier: &Barrier = barrier.downcast_ref().expect("bad barrier type");
        let data = my_barrier.data();

        let device = self.device.vk_device();
        unsafe {
            device.cmd_pipeline_barrier(
                self.vk_cmd_buffer,
                if data.src_stage_mask.is_empty() {
                    vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT
                } else {
                    data.src_stage_mask
                },
                if data.dst_stage_mask.is_empty() {
                    vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT
                } else {
                    data.dst_stage_mask
                },
                vk::DependencyFlags::empty(),
                &data.global_barriers,
                &data.buffer_barriers,
                &data.image_barriers,
            );
        } */
    }
}

#[derive(Debug)]
pub(super) struct DescSetBindingTable {
    /// The first arugment table index that needs rebinding.
    start_dirty: usize,
    table_sig_id: [usize; crate::MAX_NUM_ARG_TABLES],
    desc_sets: [vk::DescriptorSet; crate::MAX_NUM_ARG_TABLES],

    /// The root signature of the currently bound pipeline.
    bound_root_sig: Option<RootSig>,
}

impl DescSetBindingTable {
    crate fn new() -> Self {
        Self {
            start_dirty: 0,
            table_sig_id: [0; crate::MAX_NUM_ARG_TABLES],
            desc_sets: [vk::DescriptorSet::null(); crate::MAX_NUM_ARG_TABLES],
            bound_root_sig: None,
        }
    }

    crate fn bind_root_sig(&mut self, root_sig: &RootSig) {
        self.bound_root_sig = Some(root_sig.clone());
    }

    crate fn bind_arg_table(
        &mut self,
        index: base::ArgTableIndex,
        tables: &[(&base::ArgPoolRef, &base::ArgTableRef)],
    ) {
        use std::cmp::min;

        if tables.len() == 0 {
            return;
        }

        // TODO: Add reference

        for (i, (_pool, table)) in tables.iter().enumerate() {
            let my_table: &ArgTable = table.downcast_ref().expect("bad argument table type");
            self.desc_sets[i + index] = my_table.vk_descriptor_set();
        }

        self.start_dirty = min(self.start_dirty, index);
    }

    crate fn flush(
        &mut self,
        device: &DeviceRef,
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
