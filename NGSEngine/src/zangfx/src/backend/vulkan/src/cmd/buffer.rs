//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdBuffer` for Vulkan.
use ash::version::*;
use ash::vk;
use std::sync::Arc;
use std::ops::Range;
use arrayvec::ArrayVec;
use parking_lot::Mutex;

use base;
use common::{Error, ErrorKind, Result};

use device::DeviceRef;
use utils::{translate_access_type_flags, translate_generic_error_unwrap,
            translate_pipeline_stage_flags};
use buffer::Buffer;

use super::queue::{CommitedBuffer, Scheduler};
use super::enc::{FenceSet, RefTable};
use super::enc_copy::CopyEncoder;
use super::enc_compute::ComputeEncoder;

/// Implementation of `CmdBuffer` for Vulkan.
#[derive(Debug)]
pub struct CmdBuffer {
    uncommited: Option<Uncommited>,
}

zangfx_impl_object! { CmdBuffer: base::CmdBuffer, ::Debug }

#[derive(Debug)]
struct Uncommited {
    device: DeviceRef,
    scheduler: Arc<Scheduler>,
    vk_cmd_pool: vk::CommandPool,
    vk_cmd_buffer: vk::CommandBuffer,
    cmd_pool_lock: Arc<Mutex<()>>,

    fence_set: FenceSet,
    ref_table: RefTable,

    /// The set of registered completion callbacks.
    completion_callbacks: CallbackSet,

    /// Currently active encoder.
    encoder: Option<Encoder>,
}

#[derive(Debug)]
enum Encoder {
    Copy(CopyEncoder),
    Compute(ComputeEncoder),
}

#[derive(Default)]
struct CallbackSet(Vec<Box<FnMut() + Sync + Send>>);

impl ::Debug for CallbackSet {
    fn fmt(&self, f: &mut ::fmt::Formatter) -> ::fmt::Result {
        f.debug_tuple("CallbackSet")
            .field(&format!("[{} elements]", self.0.len()))
            .finish()
    }
}

impl Drop for CmdBuffer {
    fn drop(&mut self) {
        if let Some(ref uncommited) = self.uncommited {
            // This command buffer was dropped without being commited.

            let vk_device = uncommited.device.vk_device();
            unsafe {
                let _lock = uncommited.cmd_pool_lock.lock();
                vk_device.free_command_buffers(uncommited.vk_cmd_pool, &[uncommited.vk_cmd_buffer]);
            }
        }
    }
}

impl CmdBuffer {
    pub(super) fn new(
        device: DeviceRef,
        vk_cmd_pool: vk::CommandPool,
        scheduler: Arc<Scheduler>,
        cmd_pool_lock: Arc<Mutex<()>>,
    ) -> Result<Self> {
        let vk_device = device.vk_device();

        let vk_cmd_buffer = unsafe {
            let _lock = cmd_pool_lock.lock();
            vk_device.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                s_type: vk::StructureType::CommandBufferAllocateInfo,
                p_next: ::null(),
                command_pool: vk_cmd_pool,
                level: vk::CommandBufferLevel::Primary,
                command_buffer_count: 1,
            })
        }.map_err(translate_generic_error_unwrap)?[0];

        let uncommited = Uncommited {
            device,
            scheduler,
            vk_cmd_pool,
            vk_cmd_buffer,
            cmd_pool_lock,
            fence_set: FenceSet::new(),
            ref_table: RefTable::new(),
            completion_callbacks: Default::default(),
            encoder: None,
        };

        let cmd_buffer = Self {
            uncommited: Some(uncommited),
        };

        unsafe {
            vk_device.begin_command_buffer(
                cmd_buffer.uncommited.as_ref().unwrap().vk_cmd_buffer,
                &vk::CommandBufferBeginInfo {
                    s_type: vk::StructureType::CommandBufferBeginInfo,
                    p_next: ::null(),
                    flags: vk::COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                    p_inheritance_info: ::null(),
                },
            )
        }.map_err(translate_generic_error_unwrap)?;

        Ok(cmd_buffer)
    }
}

impl Uncommited {
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
            }
        }
    }
}

fn already_commited_error() -> Error {
    Error::with_detail(
        ErrorKind::InvalidUsage,
        "command buffer is already commited",
    )
}

impl base::CmdBuffer for CmdBuffer {
    fn enqueue(&mut self) -> Result<()> {
        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        // Commiting a command buffer implicitly enqueues it
        self.enqueue()?;

        {
            let uncommited = self.uncommited
                .as_mut()
                .ok_or_else(already_commited_error)
                .unwrap();

            let vk_device = uncommited.device.vk_device();

            unsafe { vk_device.end_command_buffer(uncommited.vk_cmd_buffer) }
                .map_err(translate_generic_error_unwrap)?;
        }

        let mut uncommited = self.uncommited.take().unwrap();
        uncommited.clear_encoder();

        uncommited.scheduler.commit(CommitedBuffer {
            fence_set: uncommited.fence_set,
            ref_table: Some(uncommited.ref_table),
            vk_cmd_buffer: uncommited.vk_cmd_buffer,
            completion_handler: BufferCompleteCallback {
                completion_callbacks: uncommited.completion_callbacks,
            },
        });

        Ok(())
    }

    fn encode_render(
        &mut self,
        _render_target_table: &base::RenderTargetTable,
    ) -> &mut base::RenderCmdEncoder {
        unimplemented!()
    }
    fn encode_compute(&mut self) -> &mut base::ComputeCmdEncoder {
        use std::mem::replace;

        let uncommited = self.uncommited
            .as_mut()
            .ok_or_else(already_commited_error)
            .unwrap();
        uncommited.clear_encoder();

        let encoder = unsafe {
            ComputeEncoder::new(
                uncommited.device,
                uncommited.vk_cmd_buffer,
                replace(&mut uncommited.fence_set, Default::default()),
                replace(&mut uncommited.ref_table, Default::default()),
            )
        };
        uncommited.encoder = Some(Encoder::Compute(encoder));
        match uncommited.encoder {
            Some(Encoder::Compute(ref mut e)) => e,
            _ => unreachable!(),
        }
    }
    fn encode_copy(&mut self) -> &mut base::CopyCmdEncoder {
        use std::mem::replace;

        let uncommited = self.uncommited
            .as_mut()
            .ok_or_else(already_commited_error)
            .unwrap();
        uncommited.clear_encoder();

        let encoder = unsafe {
            CopyEncoder::new(
                uncommited.device,
                uncommited.vk_cmd_buffer,
                replace(&mut uncommited.fence_set, Default::default()),
            )
        };
        uncommited.encoder = Some(Encoder::Copy(encoder));
        match uncommited.encoder {
            Some(Encoder::Copy(ref mut e)) => e,
            _ => unreachable!(),
        }
    }

    fn on_complete(&mut self, cb: Box<FnMut() + Sync + Send>) {
        let uncommited = self.uncommited
            .as_mut()
            .ok_or_else(already_commited_error)
            .unwrap();
        uncommited.completion_callbacks.0.push(cb);
    }

    fn wait_semaphore(&mut self, _semaphore: &base::Semaphore, _dst_stage: base::StageFlags) {
        unimplemented!()
    }

    fn signal_semaphore(&mut self, _semaphore: &base::Semaphore, _src_stage: base::StageFlags) {
        unimplemented!()
    }

    fn host_barrier(
        &mut self,
        src_access: base::AccessTypeFlags,
        buffers: &[(Range<base::DeviceSize>, &base::Buffer)],
    ) {
        let uncommited = self.uncommited
            .as_mut()
            .ok_or_else(already_commited_error)
            .unwrap();
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
                })
                .collect();

            unsafe {
                vk_device.cmd_pipeline_barrier(
                    uncommited.vk_cmd_buffer,
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

    fn queue_acquire_barrier(
        &mut self,
        _src_queue_family: base::QueueFamily,
        _barrier: &base::Barrier,
    ) {
        unimplemented!()
    }

    fn queue_release_barrier(
        &mut self,
        _dst_queue_family: base::QueueFamily,
        _barrier: &base::Barrier,
    ) {
        unimplemented!()
    }
}

#[derive(Debug)]
pub(crate) struct BufferCompleteCallback {
    completion_callbacks: CallbackSet,
}

impl BufferCompleteCallback {
    pub(super) fn on_complete(&mut self) {
        for mut callback in self.completion_callbacks.0.drain(..) {
            callback();
        }
    }
}
