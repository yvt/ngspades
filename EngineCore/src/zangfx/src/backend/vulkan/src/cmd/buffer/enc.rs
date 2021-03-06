//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use arrayvec::ArrayVec;
use ash::version::*;
use ash::vk;
use flags_macro::flags;
use smallvec::SmallVec;
use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use zangfx_base as base;

use crate::arg::layout::RootSig;
use crate::arg::pool::{ArgPool, ArgPoolDataRef, ArgTable};
use crate::buffer::Buffer;
use crate::cmd::fence::Fence;
use crate::device::DeviceRef;
use crate::image::{Image, ImageStateAddresser, ImageView};
use crate::limits::DeviceTraitFlags;
use crate::pipeline::{ComputePipeline, RenderPipeline};
use crate::renderpass::RenderTargetTable;
use crate::resstate::{CmdBuffer, RefTable};
use crate::utils::{translate_access_type_flags, translate_pipeline_stage_flags};

use super::super::semaphore::Semaphore;
use super::{CmdBufferData, EncodingState, Pass, PassImageBarrier};

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
        ref_entry.op.signaled = true;

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
    crate arg_pools: RefTable<ArgPoolDataRef, ()>,
    crate buffers: RefTable<Buffer, ()>,
    crate images: RefTable<Image, ImageOp>,
    crate image_views: RefTable<Arc<ImageView>, ()>,
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

/// The locally tracked state of an image for a command buffer.
#[derive(Debug, Default)]
crate struct ImageOp {
    crate units: SmallVec<[Option<ImageUnitOp>; 1]>,
}

#[derive(Debug, Clone)]
crate struct ImageUnitOp {
    /// The layout which the image is known to be.
    crate layout: vk::ImageLayout,

    /// The first value is the pass index where this layer was accessed for the
    /// last time. The second value is an index into `image_barriers`.
    ///
    /// This is only set and read by copy commands to determine if memory
    /// barriers have to be inserted between copy commands.
    crate last_pass: (usize, usize),
}

impl RefTableSet {
    crate fn new(resstate_cb: CmdBuffer) -> Self {
        Self {
            cmd_buffer: resstate_cb,
            fences: Default::default(),
            compute_pipelines: Default::default(),
            render_pipelines: Default::default(),
            render_target_tables: Default::default(),
            arg_pools: Default::default(),
            buffers: Default::default(),
            images: Default::default(),
            image_views: Default::default(),
        }
    }

    crate fn clear(&mut self) {
        self.fences.clear(&mut self.cmd_buffer, |_, _| {});
        self.arg_pools.clear(&mut self.cmd_buffer, |_, _| {});
        self.buffers.clear(&mut self.cmd_buffer, |_, _| {});
        self.images.clear(&mut self.cmd_buffer, |_, _| {});
        self.image_views.clear(&mut self.cmd_buffer, |_, _| {});
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

    crate fn insert_arg_pool(&mut self, obj: &ArgPool) {
        self.arg_pools
            .get_index_for_resource(&mut self.cmd_buffer, obj.data());
    }

    crate fn insert_buffer(&mut self, obj: &Buffer) {
        self.buffers
            .get_index_for_resource(&mut self.cmd_buffer, obj);
    }

    crate fn insert_image(&mut self, obj: &Image) -> (usize, &mut ImageOp) {
        let entry = self.images.get_mut(&mut self.cmd_buffer, obj);
        if entry.op.units.len() == 0 {
            // Initialize `ImageOp`
            let num_units = ImageStateAddresser::from_image(obj).len();
            entry.op.units.resize(num_units, None);
        }
        (entry.index, entry.op)
    }

    /// Track the lifetime of `vk::ImageView` of `Image`.
    ///
    /// Note: Image states must be handled separately.
    crate fn insert_image_view(&mut self, obj: &Image) {
        let image_view = obj.image_view();

        // There can be an `Image` referring the same `ImageState` as `obj` does
        // in `self.images`. We can return now if it also refers to the same
        // image view.
        // (One `ImageState` can be shared among multiple image views.)
        let i = self
            .images
            .try_get_index_for_resource(&mut self.cmd_buffer, obj);
        if let Some(i) = i {
            let entry = self.images.get_by_index(i);
            if Arc::ptr_eq(obj.image_view(), entry.resource.image_view()) {
                return;
            }
        }

        self.image_views
            .get_index_for_resource(&mut self.cmd_buffer, image_view);
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
        ref_table: &mut RefTableSet,
        index: base::ArgTableIndex,
        tables: &[(&base::ArgPoolRef, &base::ArgTableRef)],
    ) {
        use std::cmp::min;

        if tables.len() == 0 {
            return;
        }

        for (i, (pool, table)) in tables.iter().enumerate() {
            let my_table: &ArgTable = table.downcast_ref().expect("bad argument table type");
            self.desc_sets[i + index] = my_table.vk_descriptor_set();

            // Add the pool to the reference table
            let my_pool: &ArgPool = pool.query_ref().expect("bad argument pool type");
            ref_table.insert_arg_pool(my_pool);
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
                    s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
                    p_next: crate::null(),
                    command_pool: self.vk_cmd_pool,
                    level: vk::CommandBufferLevel::PRIMARY,
                    command_buffer_count: 1,
                })
                .map(|cbs| cbs[0])
        }
        .unwrap();
        // TODO: Handle command buffer allocation error

        unsafe {
            vk_device.begin_command_buffer(
                vk_cmd_buffer,
                &vk::CommandBufferBeginInfo {
                    s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
                    p_next: crate::null(),
                    flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                    p_inheritance_info: crate::null(),
                },
            )
        }
        .unwrap();
        // TODO: Handle command buffer beginning error

        self.passes.push(Pass {
            vk_cmd_buffer,
            signal_fences: Vec::new(),
            wait_fences: Vec::new(),
            image_barriers: Vec::new(),
            image_layout_overrides: Vec::new(),
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

        for (_, buffer) in buffers.iter() {
            let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
            self.ref_table.insert_buffer(buffer);
        }

        let vk_device = self.device.vk_device();

        let src_access_mask = translate_access_type_flags(src_access);
        let src_stages = translate_pipeline_stage_flags(src_access.supported_stages());
        for buffers in buffers.chunks(64) {
            let buf_barriers: ArrayVec<[_; 64]> = buffers
                .iter()
                .map(|&(ref range, ref buffer)| {
                    let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");

                    vk::BufferMemoryBarrier {
                        s_type: vk::StructureType::BUFFER_MEMORY_BARRIER,
                        p_next: crate::null(),
                        src_access_mask,
                        dst_access_mask: vk::AccessFlags::HOST_READ,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        buffer: my_buffer.vk_buffer(),
                        offset: range.start,
                        size: range.end - range.start,
                    }
                })
                .collect();

            unsafe {
                vk_device.cmd_pipeline_barrier(
                    self.vk_cmd_buffer(),
                    src_stages,
                    vk::PipelineStageFlags::HOST,
                    vk::DependencyFlags::empty(),
                    &[],
                    buf_barriers.as_slice(),
                    &[],
                );
            }
        }
    }

    fn queue_ownership(
        &mut self,
        src_queue_family_index: base::QueueFamily,
        dst_queue_family_index: base::QueueFamily,
        src_access_mask: vk::AccessFlags,
        dst_access_mask: vk::AccessFlags,
        src_stages: vk::PipelineStageFlags,
        dst_stages: vk::PipelineStageFlags,
        release: bool,
        transfers: &[base::QueueOwnershipTransfer<'_>],
    ) {
        use zangfx_base::QueueOwnershipTransfer;

        if self.state == EncodingState::None {
            self.begin_pass();
        }

        let vk_cmd_buffer = self.vk_cmd_buffer();
        let vk_device = self.device.vk_device();
        let current_pass = self.passes.last_mut().unwrap();

        let mut buffer_barriers = ArrayVec::<[_; 64]>::new();
        let mut image_barriers = ArrayVec::<[_; 64]>::new();

        for txs in transfers.chunks(64) {
            buffer_barriers.clear();
            image_barriers.clear();

            for tx in txs.iter() {
                match tx {
                    QueueOwnershipTransfer::Buffer { buffer, range } => {
                        let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");

                        let range = range.as_ref();

                        buffer_barriers.push(vk::BufferMemoryBarrier {
                            s_type: vk::StructureType::BUFFER_MEMORY_BARRIER,
                            p_next: crate::null(),
                            src_access_mask,
                            dst_access_mask,
                            src_queue_family_index,
                            dst_queue_family_index,
                            buffer: my_buffer.vk_buffer(),
                            offset: range.map(|r| r.start).unwrap_or(0),
                            size: range.map(|r| r.end - r.start).unwrap_or(vk::WHOLE_SIZE),
                        });
                    }
                    QueueOwnershipTransfer::Image {
                        image,
                        src_layout,
                        dst_layout,
                        range,
                    } => {
                        let image: &Image = image.downcast_ref().expect("bad image type");

                        let addresser = ImageStateAddresser::from_image(image);
                        let range = addresser.round_up_subrange(&image.resolve_subrange(range));

                        image_barriers.push(vk::ImageMemoryBarrier {
                            s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                            p_next: crate::null(),
                            src_access_mask,
                            dst_access_mask,
                            src_queue_family_index,
                            dst_queue_family_index,
                            old_layout: image.translate_layout(*src_layout),
                            new_layout: image.translate_layout(*dst_layout),
                            image: image.vk_image(),
                            subresource_range: range.to_vk_subresource_range(image.aspects()),
                        });
                    }
                }
            }

            unsafe {
                vk_device.cmd_pipeline_barrier(
                    vk_cmd_buffer,
                    src_stages,
                    dst_stages,
                    vk::DependencyFlags::empty(),
                    &[],
                    buffer_barriers.as_slice(),
                    image_barriers.as_slice(),
                );
            }
        }

        for tx in transfers.iter() {
            match tx {
                QueueOwnershipTransfer::Image {
                    image,
                    range,
                    dst_layout,
                    ..
                } => {
                    let image: &Image = image.downcast_ref().expect("bad image type");
                    let addresser = ImageStateAddresser::from_image(image);
                    let (image_index, _) = self.ref_table.insert_image(image);

                    // For each state-tracking unit...
                    let layout = if release {
                        vk::ImageLayout::UNDEFINED
                    } else {
                        image.translate_layout(*dst_layout)
                    };
                    for i in addresser.indices_for_image_and_subrange(image, range) {
                        current_pass
                            .image_layout_overrides
                            .push((image_index, i, layout));
                    }
                }
                QueueOwnershipTransfer::Buffer { buffer, .. } => {
                    let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
                    self.ref_table.insert_buffer(buffer);
                }
            }
        }
    }

    crate fn queue_ownership_acquire(
        &mut self,
        src_queue_family: base::QueueFamily,
        dst_access: base::AccessTypeFlags,
        transfer: &[base::QueueOwnershipTransfer<'_>],
    ) {
        let dst_stage = dst_access.supported_stages();

        self.queue_ownership(
            src_queue_family,
            self.queue_family,
            vk::AccessFlags::empty(),
            translate_access_type_flags(dst_access),
            vk::PipelineStageFlags::TOP_OF_PIPE,
            if dst_stage.is_empty() {
                vk::PipelineStageFlags::BOTTOM_OF_PIPE
            } else {
                translate_pipeline_stage_flags(dst_stage)
            },
            false,
            transfer,
        );
    }

    crate fn queue_ownership_release(
        &mut self,
        dst_queue_family: base::QueueFamily,
        src_access: base::AccessTypeFlags,
        transfer: &[base::QueueOwnershipTransfer<'_>],
    ) {
        let src_stage = src_access.supported_stages();

        self.queue_ownership(
            self.queue_family,
            dst_queue_family,
            translate_access_type_flags(src_access),
            vk::AccessFlags::empty(),
            if src_stage.is_empty() {
                vk::PipelineStageFlags::TOP_OF_PIPE
            } else {
                translate_pipeline_stage_flags(src_stage)
            },
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            false,
            transfer,
        );
    }

    crate fn invalidate_image(&mut self, images: &[&base::ImageRef]) {
        if self.state == EncodingState::None {
            self.begin_pass();
        }

        let current_pass = self.passes.last_mut().unwrap();

        for image in images.iter() {
            let image: &Image = image.downcast_ref().expect("bad image type");
            let addresser = ImageStateAddresser::from_image(image);
            let (image_index, _) = self.ref_table.insert_image(image);

            // For each state-tracking unit...
            for i in addresser.indices_for_image(image) {
                current_pass.image_layout_overrides.push((
                    image_index,
                    i,
                    vk::ImageLayout::UNDEFINED,
                ));
            }
        }
    }

    /// Encode `vkCmdSetEvent` to do the fence updating operation.
    crate fn cmd_update_fence(&self, fence: &Fence, src_access: base::AccessTypeFlags) {
        let traits = self.device.caps().info.traits;
        if traits.intersects(DeviceTraitFlags::MOLTEN_VK) {
            // Skip all event operations on MoltenVK
            return;
        }

        let src_stage = src_access.supported_stages();

        let device = self.device.vk_device();
        unsafe {
            device.fp_v1_0().cmd_set_event(
                self.vk_cmd_buffer(),
                fence.vk_event(),
                if src_stage.is_empty() {
                    vk::PipelineStageFlags::TOP_OF_PIPE
                } else {
                    translate_pipeline_stage_flags(src_stage)
                },
            );
        }
    }

    /// Encode necessary image layout transitions for its use within the current
    /// pass. Furthermore, add a given image to the reference table. Image
    /// layout transitions are recorded into `Pass::image_barriers`, which means
    /// they happen before all commands in the current pass.
    ///
    /// This cannot be used for copy commands that requires per-command (not
    /// just pass) tracking, unless they operate on images having a "mutable"
    /// usage flag.
    crate fn use_image_for_pass(
        &mut self,
        layout: vk::ImageLayout,
        final_layout: vk::ImageLayout,
        access: base::AccessTypeFlags,
        image: &Image,
    ) {
        let addresser = ImageStateAddresser::from_image(image);

        let (image_index, op) = self.ref_table.insert_image(image);

        let current_pass = self.passes.last_mut().unwrap();

        // For each state-tracking unit...
        for i in addresser.indices_for_image(image) {
            let current_layout;
            if let Some(ref unit_op) = op.units[i] {
                current_layout = unit_op.layout;
            } else {
                current_layout = vk::ImageLayout::UNDEFINED;
            }

            if current_layout != layout || current_layout != final_layout {
                current_pass.image_barriers.push(PassImageBarrier {
                    image_index,
                    unit_index: i,
                    initial_layout: layout,
                    final_layout,
                    access,
                });

                op.units[i] = Some(ImageUnitOp {
                    layout,
                    last_pass: (<usize>::max_value(), <usize>::max_value()),
                });
            }
        }

        self.ref_table.insert_image_view(image);
    }
}

impl base::CmdEncoder for CmdBufferData {
    fn begin_debug_group(&mut self, _label: &str) {
        // TODO: debug commands
    }

    fn end_debug_group(&mut self) {
        // TODO: debug commands
    }

    fn debug_marker(&mut self, _label: &str) {
        // TODO: debug commands
    }

    fn use_resource_core(&mut self, usage: base::ResourceUsageFlags, objs: base::ResourceSet<'_>) {
        for buffer in objs.buffers() {
            let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
            self.ref_table.insert_buffer(buffer);
        }

        // TODO: Add "access type" to the base API
        let mut access = base::AccessTypeFlags::empty();
        if usage.intersects(flags![base::ResourceUsageFlags::{READ | SAMPLE}]) {
            access |= flags![base::AccessTypeFlags::{
                VERTEX_UNIFORM_READ | VERTEX_READ | FRAGMENT_UNIFORM_READ | FRAGMENT_READ |
                COMPUTE_UNIFORM_READ | COMPUTE_READ}];
        }
        if usage.intersects(base::ResourceUsageFlags::WRITE) {
            access |= flags![base::AccessTypeFlags::{
                VERTEX_WRITE | FRAGMENT_WRITE | COMPUTE_WRITE}];
        }

        // We must use every access flags bit supported by any of render and
        // compute pass types as the destination access type because image
        // layout transition serves as an implicit write access on an image,
        // and we insert image layout barriers only when a corresponding fence
        // wait operation is defined.
        //
        // Previously we masked the destination access type with the current
        // command pass type, which turned out to be wrong. To understand why
        // this is wrong, consider the following example:
        //
        //    copy:
        //        copy_buffer_to_image(..., Image)
        //        update(Fence)
        //    render:
        //        wait(Fence)    // Layout change (Copy → Image),
        //                       // dst_access = Render
        //        use(Image)
        //    compute:
        //        wait(Fence)    // No layout change
        //        use(Image)     // BUG: Layout transiton of Image might not be
        //                       // complete at this point
        //
        // In this example, the compute pass might observe a corrupted image
        // because an image layout transition operation defined in the second
        // pass might be still in progress.
        use zangfx_base::QueueFamilyCapsFlags;
        let mut supported = base::AccessTypeFlags::empty();
        let qf_caps = self.device.caps().info.queue_families[self.queue_family as usize].caps;
        if qf_caps.contains(QueueFamilyCapsFlags::RENDER) {
            supported |= flags![base::AccessTypeFlags::{
                VERTEX_UNIFORM_READ | VERTEX_READ | FRAGMENT_UNIFORM_READ | FRAGMENT_READ |
                VERTEX_WRITE | FRAGMENT_WRITE}];
        }
        if qf_caps.contains(QueueFamilyCapsFlags::COMPUTE) {
            supported |= flags![base::AccessTypeFlags::{
                    COMPUTE_UNIFORM_READ | COMPUTE_READ | COMPUTE_WRITE}];
        }

        access &= supported;

        for image in objs.images() {
            let image: &Image = image.downcast_ref().expect("bad image type");
            let layout = image.translate_layout(base::ImageLayout::Shader);
            self.use_image_for_pass(layout, layout, access, image);
        }
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
        _obj: base::ResourceSet<'_>,
        src_access: base::AccessTypeFlags,
        dst_access: base::AccessTypeFlags,
    ) {
        let src_stage = src_access.supported_stages();
        let dst_stage = dst_access.supported_stages();

        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.cmd_pipeline_barrier(
                self.vk_cmd_buffer(),
                if src_stage.is_empty() {
                    vk::PipelineStageFlags::TOP_OF_PIPE
                } else {
                    translate_pipeline_stage_flags(src_stage)
                },
                if dst_stage.is_empty() {
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE
                } else {
                    translate_pipeline_stage_flags(dst_stage)
                },
                vk::DependencyFlags::empty(),
                &[vk::MemoryBarrier {
                    s_type: vk::StructureType::MEMORY_BARRIER,
                    p_next: crate::null(),
                    src_access_mask: translate_access_type_flags(src_access),
                    dst_access_mask: translate_access_type_flags(dst_access),
                }],
                &[],
                &[],
            );
        }
    }
}
