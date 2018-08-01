//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdBuffer` for Vulkan.
use arrayvec::ArrayVec;
use ash::version::*;
use ash::vk;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_object};

use crate::buffer::Buffer;
use crate::device::DeviceRef;
use crate::resstate;
use crate::utils::{
    translate_access_type_flags, translate_generic_error_unwrap, translate_pipeline_stage_flags,
};

use super::bufferpool::{CbPoolContent, CbPoolItem};
use super::enc::{FenceSet, RefTableSet};
use super::enc_compute::ComputeEncoder;
use super::enc_copy::CopyEncoder;
use super::enc_render::RenderEncoder;
use super::queue::Scheduler;
use super::semaphore::Semaphore;

/// Implementation of `CmdBuffer` for Vulkan.
#[derive(Debug)]
pub struct CmdBuffer {
    uncommited: Option<CbPoolItem<Box<CmdBufferData>>>,
}

zangfx_impl_object! { CmdBuffer: dyn base::CmdBuffer, dyn (crate::Debug) }

#[derive(Debug)]
crate struct CmdBufferData {
    device: DeviceRef,
    scheduler: Arc<Scheduler>,
    vk_cmd_pool: vk::CommandPool,

    crate passes: Vec<Pass>,

    crate fence_set: FenceSet,
    crate ref_table: RefTableSet,

    crate wait_semaphores: Vec<(Semaphore, vk::PipelineStageFlags)>,
    crate signal_semaphores: Vec<Semaphore>,

    /// The set of registered completion callbacks.
    crate completion_callbacks: CallbackSet,

    /// Currently active encoder.
    encoder: Option<Encoder>,
}

#[derive(Debug)]
enum Encoder {
    Copy(CopyEncoder),
    Compute(ComputeEncoder),
    Render(RenderEncoder),
}

#[derive(Default)]
crate struct CallbackSet(Vec<Box<dyn FnMut(Result<()>) + Sync + Send>>);

/// A set of commands and dependencies encoded in a single encoder. Passes also
/// define the boundaries where command patching can happen.
#[derive(Debug)]
crate struct Pass {
    crate vk_cmd_buffer: vk::CommandBuffer,
    // TODO
}

impl fmt::Debug for CallbackSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CallbackSet")
            .field(&format!("[{} elements]", self.0.len()))
            .finish()
    }
}

impl CallbackSet {
    crate fn on_complete(&mut self, result: &mut impl FnMut() -> Result<()>) {
        for mut callback in self.0.drain(..) {
            callback(result());
        }
    }
}

impl CmdBuffer {
    crate fn new(data: CbPoolItem<Box<CmdBufferData>>) -> Self {
        Self {
            uncommited: Some(data),
        }
    }

    /// Return the underlying Vulkan command buffer. Returns `None` if the
    /// command buffer is already committed (i.e. submitted to the queue).
    pub fn vk_cmd_buffer(&self) -> Option<vk::CommandBuffer> {
        self.uncommited.as_ref().map(|x| x.vk_cmd_buffer())
    }
}

impl CmdBufferData {
    crate fn new(
        device: DeviceRef,
        queue_family_index: u32,
        scheduler: Arc<Scheduler>,
        resstate_cb: resstate::CmdBuffer,
    ) -> Result<Self> {
        let vk_cmd_pool = unsafe {
            let vk_device = device.vk_device();
            vk_device.create_command_pool(
                &vk::CommandPoolCreateInfo {
                    s_type: vk::StructureType::CommandPoolCreateInfo,
                    p_next: crate::null(),
                    flags: vk::COMMAND_POOL_CREATE_TRANSIENT_BIT,
                    queue_family_index,
                },
                None,
            )
        }.map_err(translate_generic_error_unwrap)?;

        Ok(CmdBufferData {
            device: device.clone(),
            scheduler,
            vk_cmd_pool,
            passes: Vec::new(),
            fence_set: FenceSet::new(),
            ref_table: RefTableSet::new(resstate_cb),
            wait_semaphores: Vec::new(),
            signal_semaphores: Vec::new(),
            completion_callbacks: Default::default(),
            encoder: None,
        })
    }

    /// Clear `self.encoder` and take `fence_set` back from it.
    fn clear_encoder(&mut self) {
        if let Some(enc) = self.encoder.take() {
            match enc {
                Encoder::Copy(e) => self.fence_set = e.finish(),
                Encoder::Compute(e) => {
                    let (fence_set, ref_table) = e.finish();
                    self.fence_set = fence_set;
                    self.ref_table = ref_table;
                }
                Encoder::Render(e) => {
                    let (fence_set, ref_table) = e.finish();
                    self.fence_set = fence_set;
                    self.ref_table = ref_table;
                }
            }
        }
    }

    fn vk_cmd_buffer(&self) -> vk::CommandBuffer {
        unimplemented!()
    }

    crate fn reset(&mut self) {
        let vk_device = self.device.vk_device();
        for pass in self.passes.drain(..) {
            unsafe {
                vk_device.free_command_buffers(self.vk_cmd_pool, &[pass.vk_cmd_buffer]);
            }
        }

        self.fence_set.wait_fences.clear();
        self.fence_set.signal_fences.clear();
        self.ref_table.clear();
        self.wait_semaphores.clear();
        self.signal_semaphores.clear();
        self.completion_callbacks.0.clear();

        // TODO
    }
}

impl CbPoolContent for CmdBufferData {
    /// Called when `CmdBufferData` is returned to a pool.
    fn reset(&mut self) {
        self.reset()
    }
}

impl Drop for CmdBufferData {
    fn drop(&mut self) {
        let vk_device = self.device.vk_device();
        unsafe {
            // This operation automatically frees all command buffers allocated
            // from the pool
            vk_device.destroy_command_pool(self.vk_cmd_pool, None);
        }
    }
}

impl base::CmdBuffer for CmdBuffer {
    fn commit(&mut self) -> Result<()> {
        {
            let uncommited = self
                .uncommited
                .as_mut()
                .expect("command buffer is already commited");

            uncommited.clear_encoder();

            let vk_device = uncommited.device.vk_device();

            unsafe { vk_device.end_command_buffer(uncommited.vk_cmd_buffer()) }
                .map_err(translate_generic_error_unwrap)?;
        }

        let uncommited = self.uncommited.take().unwrap();
        let scheduler = uncommited.scheduler.clone();

        scheduler.commit(uncommited);

        Ok(())
    }

    fn encode_render(
        &mut self,
        render_target_table: &base::RenderTargetTableRef,
    ) -> &mut dyn base::RenderCmdEncoder {
        use crate::renderpass::RenderTargetTable;
        use std::mem::replace;

        let rtt: &RenderTargetTable = render_target_table
            .downcast_ref()
            .expect("bad render target table type");

        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
        uncommited.clear_encoder();

        let encoder = unsafe {
            RenderEncoder::new(
                uncommited.device.clone(),
                uncommited.vk_cmd_buffer(),
                unimplemented!(), // replace(&mut uncommited.fence_set, Default::default()),
                unimplemented!(), // replace(&mut uncommited.ref_table, Default::default()),
                rtt,
            )
        };
        uncommited.encoder = Some(Encoder::Render(encoder));
        match uncommited.encoder {
            Some(Encoder::Render(ref mut e)) => e,
            _ => unreachable!(),
        }
    }
    fn encode_compute(&mut self) -> &mut dyn base::ComputeCmdEncoder {
        use std::mem::replace;

        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
        uncommited.clear_encoder();

        let encoder = unsafe {
            ComputeEncoder::new(
                uncommited.device.clone(),
                uncommited.vk_cmd_buffer(),
                unimplemented!(), // replace(&mut uncommited.fence_set, Default::default()),
                unimplemented!(), // replace(&mut uncommited.ref_table, Default::default()),
            )
        };
        uncommited.encoder = Some(Encoder::Compute(encoder));
        match uncommited.encoder {
            Some(Encoder::Compute(ref mut e)) => e,
            _ => unreachable!(),
        }
    }
    fn encode_copy(&mut self) -> &mut dyn base::CopyCmdEncoder {
        use std::mem::replace;

        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
        uncommited.clear_encoder();

        let encoder = unsafe {
            CopyEncoder::new(
                uncommited.device.clone(),
                uncommited.vk_cmd_buffer(),
                unimplemented!(), // replace(&mut uncommited.fence_set, Default::default()),
            )
        };
        uncommited.encoder = Some(Encoder::Copy(encoder));
        match uncommited.encoder {
            Some(Encoder::Copy(ref mut e)) => e,
            _ => unreachable!(),
        }
    }

    fn on_complete(&mut self, cb: Box<dyn FnMut(Result<()>) + Sync + Send>) {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");

        uncommited.completion_callbacks.0.push(cb);
    }

    fn wait_semaphore(&mut self, semaphore: &base::SemaphoreRef, dst_stage: base::StageFlags) {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
        let our_semaphore = Semaphore::clone(semaphore.downcast_ref().expect("bad semaphore type"));
        let stage = translate_pipeline_stage_flags(dst_stage);
        uncommited.wait_semaphores.push((our_semaphore, stage));
    }

    fn signal_semaphore(&mut self, semaphore: &base::SemaphoreRef, _src_stage: base::StageFlags) {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
        let our_semaphore = Semaphore::clone(semaphore.downcast_ref().expect("bad semaphore type"));
        uncommited.signal_semaphores.push(our_semaphore);
    }

    fn host_barrier(
        &mut self,
        src_access: base::AccessTypeFlags,
        buffers: &[(Range<base::DeviceSize>, &base::BufferRef)],
    ) {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
        uncommited.clear_encoder();

        let vk_device = uncommited.device.vk_device();

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
                    uncommited.vk_cmd_buffer(),
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

    fn queue_ownership_acquire(
        &mut self,
        _src_queue_family: base::QueueFamily,
        _dst_access: base::AccessTypeFlags,
        _transfer: &base::QueueOwnershipTransfer<'_>,
    ) {
        unimplemented!()
    }

    fn queue_ownership_release(
        &mut self,
        _dst_queue_family: base::QueueFamily,
        _src_access: base::AccessTypeFlags,
        _transfer: &base::QueueOwnershipTransfer<'_>,
    ) {
        unimplemented!()
    }
}
