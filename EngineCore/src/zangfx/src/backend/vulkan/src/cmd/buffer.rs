//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdBuffer` for Vulkan.
use ash::version::*;
use ash::vk;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_object};

use crate::device::DeviceRef;
use crate::resstate;
use crate::utils::translate_generic_error_unwrap;

use super::bufferpool::{CbPoolContent, CbPoolItem};
use super::queue::Scheduler;
use super::semaphore::Semaphore;

mod enc;
mod enc_compute;
mod enc_copy;
mod enc_render;
mod patch;

use self::enc::{DescSetBindingTable, FenceSet, RefTableSet};

/// Implementation of `CmdBuffer` for Vulkan.
#[derive(Debug)]
pub struct CmdBuffer {
    uncommited: Option<CbPoolItem<Box<CmdBufferData>>>,
}

zangfx_impl_object! { CmdBuffer: dyn base::CmdBuffer, dyn (crate::Debug) }

/// Stores the state of a command buffer, whether it is currently being
/// encoded or not.
///
/// This type implements the `*CmdEncoder` traits. `CmdBufferData` is accessed
/// via `&mut dyn (Copy|Render|Compute)?CmdEncoder` only when it is being
/// encoded.
/// See the `enc`, `enc_compute`, `enc_copy`, and `enc_render` modules for code
/// relevant to command buffer encoding.
///
/// Some fields are not used after encoding is done. They are reused after
/// a command buffer is returned to a pool and is allocated again.
#[derive(Debug)]
crate struct CmdBufferData {
    device: DeviceRef,
    scheduler: Arc<Scheduler>,
    vk_cmd_pool: vk::CommandPool,

    crate passes: Vec<Pass>,

    /// The optional command buffer which is executed before passes. This may be
    /// generated by `CmdBufferData::finalize`.
    crate vk_prelude_cmd_buffer: Option<vk::CommandBuffer>,

    crate fence_set: FenceSet,
    crate ref_table: RefTableSet,

    crate wait_semaphores: Vec<(Semaphore, vk::PipelineStageFlags)>,
    crate signal_semaphores: Vec<Semaphore>,

    /// The set of registered completion callbacks.
    crate completion_callbacks: CallbackSet,

    temp: CmdBufferTemp,

    /*
     * The following fields are used only when encoding
     */
    /// The current encoding state.
    state: EncodingState,

    /// Manages bound descriptor sets.
    desc_set_binding_table: DescSetBindingTable,

    /// A list of fences to be signaled after the current render pass is done.
    /// (`vkCmdSetEvent` is invalid inside a render pass.)
    deferred_signal_fences: Vec<(usize, base::AccessTypeFlags)>,
}

zangfx_impl_object! {
    CmdBufferData:
        dyn base::CmdEncoder,
        dyn base::RenderCmdEncoder,
        dyn base::CopyCmdEncoder,
        dyn base::ComputeCmdEncoder,
        dyn (crate::Debug)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum EncodingState {
    None,

    /// We are currently encoding `passes.last().unwrap()` and we are outside a
    /// render pass.
    NotRender,

    /// We are currently encoding `passes.last().unwrap()` and we are inside a
    /// render pass.
    Render,
}

/// Temporary variables.
#[derive(Debug, Default)]
crate struct CmdBufferTemp {
    vk_image_barriers: Vec<vk::ImageMemoryBarrier>,
}

unsafe impl Send for CmdBufferTemp {}
unsafe impl Sync for CmdBufferTemp {}

#[derive(Default)]
crate struct CallbackSet(Vec<Box<dyn FnMut(Result<()>) + Sync + Send>>);

/// A set of commands and dependencies encoded in a single encoder. Passes also
/// define the boundaries where command patching can happen.
///
/// Note: The scheduler works on the per-`CmdBuffer` basis. It's not aware of
/// `Pass`es.
#[derive(Debug)]
crate struct Pass {
    crate vk_cmd_buffer: vk::CommandBuffer,

    /// A set of fence that must be signaled before executing the pass.
    ///
    /// Each entry is an index into `RefTableSet::fences`.
    crate wait_fences: Vec<(usize, base::AccessTypeFlags)>,

    /// A set of fence that will be signaled while executing the pass.
    ///
    /// Each entry is an index into `RefTableSet::fences`.
    crate signal_fences: Vec<(usize, base::AccessTypeFlags)>,

    crate image_barriers: Vec<PassImageBarrier>,
}

/// Represents a layout transition of an image before/after a pass.
#[derive(Debug)]
crate struct PassImageBarrier {
    /// An index into `RefTableSet::images`.
    crate image_index: usize,

    /// An index representing a state-tracking unit
    /// (as defined by `ImageStateAddresser`).
    crate unit_index: usize,

    /// The layout which the pass expects the image layer to be in.
    ///
    /// `patch.rs` will insert image memory barriers to ensure that the image
    /// is in this layout when the pass is executed.
    crate initial_layout: vk::ImageLayout,

    /// The layout of the image layer after the execution of the pass.
    crate final_layout: vk::ImageLayout,
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
    pub fn vk_cmd_buffer(&mut self) -> Option<vk::CommandBuffer> {
        let uncommited = self.uncommited.as_mut()?;
        if uncommited.state == EncodingState::Render {
            uncommited.end_render_pass();
        } else if uncommited.state == EncodingState::None {
            uncommited.begin_pass();
        }
        Some(uncommited.vk_cmd_buffer())
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
            vk_prelude_cmd_buffer: None,
            fence_set: FenceSet::new(),
            ref_table: RefTableSet::new(resstate_cb),
            wait_semaphores: Vec::new(),
            signal_semaphores: Vec::new(),
            completion_callbacks: Default::default(),
            state: EncodingState::None,
            desc_set_binding_table: DescSetBindingTable::new(),
            deferred_signal_fences: Vec::new(),
            temp: Default::default(),
        })
    }

    crate fn reset_completion_callbacks(&mut self) {
        self.completion_callbacks.0.clear();
    }

    crate fn reset_all_but_completion_callbacks(&mut self) {
        self.end_pass();
        self.deferred_signal_fences.clear();
        self.desc_set_binding_table.reset();

        let vk_device = self.device.vk_device();
        for pass in self.passes.drain(..) {
            unsafe {
                vk_device.free_command_buffers(self.vk_cmd_pool, &[pass.vk_cmd_buffer]);
            }
        }

        if let Some(x) = self.vk_prelude_cmd_buffer.take() {
            unsafe {
                vk_device.free_command_buffers(self.vk_cmd_pool, &[x]);
            }
        }

        self.fence_set.wait_fences.clear();
        self.fence_set.signal_fences.clear();
        self.ref_table.clear();
        self.wait_semaphores.clear();
        self.signal_semaphores.clear();
    }

    crate fn reset(&mut self) {
        self.reset_all_but_completion_callbacks();
        self.reset_completion_callbacks();
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
        let mut uncommited = self
            .uncommited
            .take()
            .expect("command buffer is already commited");
        uncommited.end_pass();

        let scheduler = uncommited.scheduler.clone();

        scheduler.commit(uncommited);

        Ok(())
    }

    fn encode_render(
        &mut self,
        render_target_table: &base::RenderTargetTableRef,
    ) -> &mut dyn base::RenderCmdEncoder {
        use crate::renderpass::RenderTargetTable;

        let rtt: &RenderTargetTable = render_target_table
            .downcast_ref()
            .expect("bad render target table type");

        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");

        uncommited.begin_pass();
        uncommited.begin_render_pass(rtt);

        &mut ***uncommited
    }
    fn encode_compute(&mut self) -> &mut dyn base::ComputeCmdEncoder {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");

        uncommited.begin_pass();

        &mut ***uncommited
    }
    fn encode_copy(&mut self) -> &mut dyn base::CopyCmdEncoder {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");

        uncommited.begin_pass();

        &mut ***uncommited
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
        let our_semaphore = semaphore.downcast_ref().expect("bad semaphore type");
        uncommited.wait_semaphore(our_semaphore, dst_stage);
    }

    fn signal_semaphore(&mut self, semaphore: &base::SemaphoreRef, src_stage: base::StageFlags) {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
        let our_semaphore = semaphore.downcast_ref().expect("bad semaphore type");
        uncommited.signal_semaphore(our_semaphore, src_stage);
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
        uncommited.host_barrier(src_access, buffers);
    }

    fn queue_ownership_acquire(
        &mut self,
        src_queue_family: base::QueueFamily,
        dst_access: base::AccessTypeFlags,
        transfer: &base::QueueOwnershipTransfer<'_>,
    ) {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
        uncommited.queue_ownership_acquire(src_queue_family, dst_access, transfer)
    }

    fn queue_ownership_release(
        &mut self,
        dst_queue_family: base::QueueFamily,
        src_access: base::AccessTypeFlags,
        transfer: &base::QueueOwnershipTransfer<'_>,
    ) {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
        uncommited.queue_ownership_release(dst_queue_family, src_access, transfer)
    }
}
