//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use arrayvec::ArrayVec;
use ash::version::*;
use ash::vk;
use std::collections::HashSet;
use std::ops::Range;

use zangfx_base as base;

use crate::arg::layout::RootSig;
use crate::arg::pool::ArgTable;
use crate::buffer::Buffer;
use crate::cmd::fence::Fence;
use crate::device::DeviceRef;
use crate::limits::DeviceTrait;
use crate::pipeline::{ComputePipeline, RenderPipeline};
use crate::renderpass::RenderTargetTable;
use crate::resstate::{CmdBuffer, RefTable};
use crate::utils::{translate_access_type_flags, translate_pipeline_stage_flags};

use super::super::semaphore::Semaphore;
use super::{CmdBufferData, EncodingState, Pass};

#[derive(Debug, Default)]
crate struct FenceSet {
    /// A set of fence that must be signaled before executing the command
    /// buffer.
    ///
    /// Fences which are signaled by the same command buffer are not included.
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

        debug_assert!(!ref_entry.op.signaled, "fence is already signaled");

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
crate struct DescSetBindingTable {
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

    crate fn reset(&mut self) {
        *self = Self::new();
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

impl CmdBufferData {
    /// Return the underlying Vulkan command buffer for the pass that is
    /// currently being encoded.
    crate fn vk_cmd_buffer(&self) -> vk::CommandBuffer {
        debug_assert_ne!(self.state, EncodingState::None);
        self.passes.last().unwrap().vk_cmd_buffer
    }

    /// Start a new pass. Terminate the current one (if any).
    crate fn begin_pass(&mut self) {
        self.end_pass();

        self.passes.reserve(1);

        let vk_device = self.device.vk_device();

        let vk_cmd_buffer = unsafe {
            vk_device
                .allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                    s_type: vk::StructureType::CommandBufferAllocateInfo,
                    p_next: crate::null(),
                    command_pool: self.vk_cmd_pool,
                    level: vk::CommandBufferLevel::Primary,
                    command_buffer_count: 1,
                }).map(|cbs| cbs[0])
        }.unwrap();
        // TODO: Handle command buffer allocation error

        unsafe {
            vk_device.begin_command_buffer(
                vk_cmd_buffer,
                &vk::CommandBufferBeginInfo {
                    s_type: vk::StructureType::CommandBufferBeginInfo,
                    p_next: ::null(),
                    flags: vk::COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                    p_inheritance_info: ::null(),
                },
            )
        }.unwrap();
        // TODO: Handle command buffer beginning error

        self.passes.push(Pass {
            vk_cmd_buffer,
            signal_fences: Vec::new(),
            wait_fences: Vec::new(),
        });
        self.state = EncodingState::NotRender;

        self.desc_set_binding_table.reset();
    }

    /// Terminate the current pass (if any).
    crate fn end_pass(&mut self) {
        if self.state == EncodingState::Render {
            self.end_render_pass();
        }
        if self.state == EncodingState::NotRender {
            // Do not call `end_command_buffer` here
            self.state = EncodingState::None;
        }
    }

    crate fn wait_semaphore(&mut self, semaphore: &Semaphore, dst_stage: base::StageFlags) {
        let stage = translate_pipeline_stage_flags(dst_stage);
        self.wait_semaphores.push((semaphore.clone(), stage));
    }

    crate fn signal_semaphore(&mut self, semaphore: &Semaphore, _src_stage: base::StageFlags) {
        self.signal_semaphores.push(semaphore.clone());
    }

    crate fn host_barrier(
        &mut self,
        src_access: base::AccessTypeFlags,
        buffers: &[(Range<base::DeviceSize>, &base::BufferRef)],
    ) {
        if self.state == EncodingState::None {
            self.begin_pass();
        }

        let vk_device = self.device.vk_device();

        let src_access_mask = translate_access_type_flags(src_access);
        let src_stages =
            translate_pipeline_stage_flags(base::AccessType::union_supported_stages(src_access));
        for buffers in buffers.chunks(64) {
            let buf_barriers: ArrayVec<[_; 64]> = buffers
                .iter()
                .map(|&(ref range, ref buffer)| {
                    let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
                    vk::BufferMemoryBarrier {
                        s_type: vk::StructureType::BufferMemoryBarrier,
                        p_next: ::null(),
                        src_access_mask,
                        dst_access_mask: vk::ACCESS_HOST_READ_BIT,
                        src_queue_family_index: vk::VK_QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::VK_QUEUE_FAMILY_IGNORED,
                        buffer: my_buffer.vk_buffer(),
                        offset: range.start,
                        size: range.end - range.start,
                    }
                }).collect();

            unsafe {
                vk_device.cmd_pipeline_barrier(
                    self.vk_cmd_buffer(),
                    src_stages,
                    vk::PIPELINE_STAGE_HOST_BIT,
                    vk::DependencyFlags::empty(),
                    &[],
                    buf_barriers.as_slice(),
                    &[],
                );
            }
        }
    }

    crate fn queue_ownership_acquire(
        &mut self,
        _src_queue_family: base::QueueFamily,
        _dst_access: base::AccessTypeFlags,
        _transfer: &base::QueueOwnershipTransfer<'_>,
    ) {
        unimplemented!()
    }

    crate fn queue_ownership_release(
        &mut self,
        _dst_queue_family: base::QueueFamily,
        _src_access: base::AccessTypeFlags,
        _transfer: &base::QueueOwnershipTransfer<'_>,
    ) {
        unimplemented!()
    }

    /// Encode `vkCmdSetEvent` to do the fence updating operation.
    crate fn cmd_update_fence(&self, fence: &Fence, src_access: base::AccessTypeFlags) {
        let traits = self.device.caps().info.traits;
        if traits.intersects(DeviceTrait::MoltenVK) {
            // Skip all event operations on MoltenVK
            return;
        }

        let src_stage = base::AccessType::union_supported_stages(src_access);

        let device = self.device.vk_device();
        unsafe {
            device.fp_v1_0().cmd_set_event(
                self.vk_cmd_buffer(),
                fence.vk_event(),
                if src_stage.is_empty() {
                    vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT
                } else {
                    translate_pipeline_stage_flags(src_stage)
                },
            );
        }
    }
}

impl base::CmdEncoder for CmdBufferData {
    fn begin_debug_group(&mut self, label: &str) {
        // TODO: debug commands
    }

    fn end_debug_group(&mut self) {
        // TODO: debug commands
    }

    fn debug_marker(&mut self, label: &str) {
        // TODO: debug commands
    }

    fn use_resource_core(
        &mut self,
        _usage: base::ResourceUsageFlags,
        _objs: base::ResourceSet<'_>,
    ) {
        unimplemented!()
    }

    fn use_heap(&mut self, _heaps: &[&base::HeapRef]) {
        unimplemented!()
    }

    fn wait_fence(&mut self, fence: &base::FenceRef, dst_access: base::AccessTypeFlags) {
        let our_fence = fence.downcast_ref().expect("bad fence type");

        // 1. Add the fence to the reference table
        // 2. A fence describes a inter-command buffer depdendency which has
        //    a significance on command buffer scheduling.
        self.fence_set.wait_fence(&mut self.ref_table, our_fence);

        // 3. A fence describes a inter-pass dependency.
        let fence_index = {
            let ref mut ref_table = self.ref_table;
            ref_table
                .fences
                .get_index_for_resource(&mut ref_table.cmd_buffer, our_fence)
        };

        {
            let current_pass = self.passes.last_mut().unwrap();
            current_pass.wait_fences.push((fence_index, dst_access));
        }

        // 4. `vkCmdWaitEvents` is inserted during command buffer submission.
        //    We don't know the source stage flags at this point yet.
    }

    fn update_fence(&mut self, fence: &base::FenceRef, src_access: base::AccessTypeFlags) {
        let our_fence = fence.downcast_ref().expect("bad fence type");

        // 1. Add the fence to the reference table
        // 2. A fence describes a inter-command buffer depdendency which has
        //    a significance on command buffer scheduling.
        self.fence_set.signal_fence(&mut self.ref_table, our_fence);

        // 3. A fence describes a inter-pass dependency.
        let fence_index = {
            let ref mut ref_table = self.ref_table;
            ref_table
                .fences
                .get_index_for_resource(&mut ref_table.cmd_buffer, our_fence)
        };

        {
            let current_pass = self.passes.last_mut().unwrap();
            current_pass.signal_fences.push((fence_index, src_access));
        }

        // 4. Insert `vkCmdSetEvent`.
        if self.state == EncodingState::Render {
            // `vkCmdSetEvent` is not allowed inside a render pass
            self.deferred_signal_fences.push((fence_index, src_access));
        } else {
            self.cmd_update_fence(our_fence, src_access);
        }
    }

    fn barrier_core(
        &mut self,
        obj: base::ResourceSet<'_>,
        src_access: base::AccessTypeFlags,
        dst_access: base::AccessTypeFlags,
    ) {
        unimplemented!()
    }
}
