//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;
use std::sync::{Arc, Mutex};
use std::ops::Range;
use std::ptr;
use ngsgfx_common::int::BinaryInteger;

use {DeviceRef, Backend, AshDevice, translate_generic_error_unwrap};
use imp::{CommandBuffer, Event, Fence, DeviceData, DeviceConfig, MAX_NUM_QUEUES};
use utils::{translate_pipeline_stage_flags, translate_access_type_flags};
use super::encoder::EncoderState;
use super::tokenlock::{Token, TokenRef};
use super::queuesched::QueueScheduler;
use super::event::{CommandDependencyTable, LlFence};
use super::fence::{FenceQueueData, FenceWaitState};

pub struct CommandQueue<T: DeviceRef> {
    data: Box<CommandQueueData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for CommandQueue<T> => data
}

#[derive(Debug)]
struct CommandQueueData<T: DeviceRef> {
    device_data: Arc<DeviceData<T>>,
    excl_data: Mutex<CommandQueueExclData<T>>,
    token_ref: TokenRef,
    queues: Vec<vk::Queue>,
}

#[derive(Debug)]
struct CommandQueueExclData<T: DeviceRef> {
    token: Token,
    lost: bool,

    dep_table: CommandDependencyTable<T>,
}

impl<T: DeviceRef> CommandQueue<T> {
    pub(crate) fn new(device_data: &Arc<DeviceData<T>>) -> Self {
        let queues = device_data
            .cfg
            .queues
            .iter()
            .map(|&(qf, i)| unsafe {
                device_data.device_ref.device().get_device_queue(qf, i)
            })
            .collect();
        let excl_data = CommandQueueExclData {
            token: Token::new(),
            lost: false,

            dep_table: CommandDependencyTable::new(),
        };
        Self {
            data: Box::new(CommandQueueData {
                device_data: device_data.clone(),
                token_ref: TokenRef::from(&excl_data.token),
                excl_data: Mutex::new(excl_data),
                queues,
            }),
        }
    }

    pub(super) fn device_config(&self) -> &DeviceConfig {
        &self.data.device_data.cfg
    }

    pub(super) fn token_ref(&self) -> &TokenRef {
        &self.data.token_ref
    }

    pub(super) fn device_ref(&self) -> &T {
        &self.data.device_data.device_ref
    }
}

#[derive(Debug)]
struct CommandSender<T: DeviceRef> {
    device_ref: T,

    /// A list of `CommandBuffer`s for each internal queue
    cbs: Vec<Vec<vk::CommandBuffer>>,

    /// A range of semaphores to be waited for or signalled by
    /// the corresponding `CommandBuffer` in `cbs`, for each internal queue.
    /// Every range in `sem_ranges[iq]` specifies indices in `sems[iq]`.
    sem_ranges: Vec<Vec<(Range<usize>, Range<usize>)>>,

    /// A list of `Semaphore`s for each internal queue
    sems: Vec<Vec<vk::Semaphore>>,

    /// The starting index into `cbs[iq]` for the next batch submitted to
    /// the internal queue `iq`
    iq_next_cb: Vec<usize>,

    batches: Vec<Batch>,

    panicking: bool,
}

/// Represents a single submission to a Vulkan command queue.
#[derive(Debug)]
struct Batch {
    /// Internal queue index
    iq: usize,

    /// Range of `CommandBuffer`s in `cbs[iq]` to be included in this batch
    cb_range: Range<usize>,
}

impl<T: DeviceRef> Drop for CommandSender<T> {
    fn drop(&mut self) {
        if self.panicking {
            let device: &AshDevice = self.device_ref.device();

            // Ignore errors since we really can't do anything with that
            let _ = device.device_wait_idle();
        }
    }
}

impl<T: DeviceRef> CommandSender<T> {
    fn flush_iq(&mut self, iq: usize) {
        let range = self.iq_next_cb[iq]..self.cbs[iq].len();
        if range.len() == 0 {
            return;
        }
        self.batches.push(Batch {
            iq,
            cb_range: range,
        });
        self.iq_next_cb[iq] = self.cbs[iq].len();
    }

    fn submit(
        queue: &CommandQueue<T>,
        buffers: &mut [&mut CommandBuffer<T>],
        event: Option<&Event<T>>,
    ) -> core::Result<()> {
        let ref data = *queue.data;
        let mut excl_data = data.excl_data.lock().unwrap();

        let device: &AshDevice = data.device_data.device_ref.device();

        if excl_data.lost {
            return Err(core::GenericError::DeviceLost);
        }

        // Validate the command buffer encoder states
        // (We are doing this beforehand for exception safety)
        for buffer in buffers.iter() {
            match buffer.data.encoder_state {
                EncoderState::End => {}
                EncoderState::Submitted => {
                    panic!("command buffer must be recorded before being sent again");
                }
                EncoderState::Error(e) => {
                    return Err(e);
                }
                EncoderState::Initial => {
                    panic!("command buffer is not encoded");
                }
                ref x => {
                    panic!("command buffer is still in the Encoding state: {:?}", x);
                }
            }

            // And then `Fence` ownerships
            // (`queue_data_write` only checks if we are allowed to access
            // the contained data; it does not write anything by itself)
            for pass in buffer.data.passes.iter() {
                for fence in pass.wait_fences.iter() {
                    fence.0.queue_data_write(&mut excl_data.token);

                    // It is already guaranteed that waits on these fences from
                    // the internal queue `pass.internal_queue_index` are valid.
                }
                for fence in pass.update_fences.iter() {
                    fence.0.queue_data_write(&mut excl_data.token);

                    // We do not impose any restrictions on which internal queue
                    // can be used to update fences.
                    // IQ can update any fences, regardless of the value of
                    // `FenceDescription::update_engines`.
                }
            }
        }

        let num_iqs = data.device_data.cfg.queues.len();
        let mut s = CommandSender {
            device_ref: data.device_data.device_ref.clone(),
            cbs: vec![Vec::new(); num_iqs],
            sem_ranges: vec![Vec::new(); num_iqs],
            sems: vec![Vec::new(); num_iqs],
            iq_next_cb: vec![0; num_iqs],
            batches: Vec::new(),
            panicking: true,
        };
        let mut sched = QueueScheduler::new();

        // From this point on, panics are unrecoverable error
        // (Mainly because we are updating `FenceQueueData`s)
        excl_data.lost = true;

        use self::FenceWaitState::*;

        for buffer in buffers.iter_mut() {
            let ref mut buffer_data = *buffer.data;
            for pass in buffer_data.passes.iter_mut() {
                let iq = pass.internal_queue_index;

                // Check inter-queue dependency first
                let mut dep_bits = 0u32;
                let mut intra_stage_dep = core::PipelineStageFlags::empty();
                let mut intra_access_dep = core::AccessTypeFlags::empty();
                let mut intra_stage_src = core::PipelineStageFlags::empty();
                let mut intra_access_src = core::AccessTypeFlags::empty();
                let mut intra_stage_dst = core::PipelineStageFlags::empty();
                let mut intra_access_dst = core::AccessTypeFlags::empty();

                for fence in pass.wait_fences.iter() {
                    let fqd: &mut FenceQueueData<_> =
                        fence.0.queue_data_write(&mut excl_data.token);
                    match fqd.wait_states[iq] {
                        PipelineBarrier {
                            ref mut dst_scope,
                            ref src_scope,
                        } => {
                            // (or-ing both kind of dependency flags is a little bit over-conservative,
                            // but now we only need one pipeline barrier command)
                            intra_stage_src = intra_stage_src | src_scope.0;
                            intra_access_src = intra_access_src | src_scope.1;
                            intra_stage_dep = intra_stage_dep | !(fence.1 & !dst_scope.0);
                            intra_access_dep = intra_access_dep | !(fence.2 & !dst_scope.1);

                            intra_stage_dst = intra_stage_dst | fence.1;
                            intra_access_dst = intra_access_dst | fence.2;

                            dst_scope.0 = dst_scope.0 | fence.1;
                            dst_scope.1 = dst_scope.1 | fence.2;
                        }
                        Semaphore { signalled_by } => {
                            dep_bits.set_bit(signalled_by);
                        }
                        _ => {}
                    }
                }

                // Resolve circular references if any
                if dep_bits.count_ones() == 0 || !sched.insert(iq as u32, dep_bits) {
                    // Try to find the offending dependencies one by one
                    for to_iq in dep_bits.one_digits() {
                        if !sched.insert(iq as u32, 1u32 << to_iq) {
                            sched.resolve(to_iq as u32, |iq_u32| { s.flush_iq(iq_u32 as usize); });
                        }
                    }
                }

                // Assemble the list of semaphores to wait
                let mut sem_start = s.sems[iq].len();
                for fence in pass.wait_fences.iter() {
                    let fqd: &mut FenceQueueData<_> =
                        fence.0.queue_data_write(&mut excl_data.token);
                    match fqd.wait_states[iq] {
                        Semaphore { .. } => {
                            fqd.wait_states[iq] = Ready;
                            s.sems[iq].push(fence.0.get_semaphore(iq));
                        }
                        _ => {}
                    }
                }

                // Insert pipeline barriers
                if !intra_stage_dep.is_empty() || !intra_access_dep.is_empty() {
                    // Wouldn't ya mind if I borrow your command pool, would ya?
                    // TODO: safer error handling
                    let cb = unsafe {
                        buffer_data
                            .pools
                            .lock_host_write()
                            .get_mut(iq)
                            .get_primary_buffer(device)?
                    };
                    unsafe {
                        device.begin_command_buffer(
                            cb,
                            &vk::CommandBufferBeginInfo {
                                s_type: vk::StructureType::CommandBufferBeginInfo,
                                p_next: ptr::null(),
                                flags: vk::COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                                p_inheritance_info: ptr::null(),
                            },
                        ).map_err(translate_generic_error_unwrap)?;
                        device.cmd_pipeline_barrier(
                            cb,
                            translate_pipeline_stage_flags(intra_stage_src),
                            translate_pipeline_stage_flags(intra_stage_dst),
                            vk::DependencyFlags::empty(),
                            &[
                                vk::MemoryBarrier {
                                    s_type: vk::StructureType::MemoryBarrier,
                                    p_next: ptr::null(),
                                    src_access_mask: translate_access_type_flags(intra_access_src),
                                    dst_access_mask: translate_access_type_flags(intra_access_dst),
                                },
                            ],
                            &[],
                            &[],
                        );
                        device.end_command_buffer(cb)
                            .map_err(translate_generic_error_unwrap)?;
                    }

                    s.cbs[iq].push(cb);
                    s.sem_ranges[iq].push((
                        sem_start..s.sems[iq].len(),
                        s.sems[iq].len()..s.sems[iq].len(),
                    ));
                    sem_start = s.sems[iq].len();
                }

                // Deal with "loose" semaphores
                for fence in pass.update_fences.iter() {
                    let fqd: &mut FenceQueueData<_> =
                        fence.0.queue_data_write(&mut excl_data.token);
                    for (to_iq, wait_state) in fqd.wait_states.iter_mut().enumerate() {
                        match wait_state {
                            &mut Semaphore { .. } => {
                                // This semaphore is to be signaled, but no one
                                // wanted to signal it.
                                // It cannot be signaled again until it is waited
                                // by someone.
                                *wait_state = Ready;
                                s.sems[iq].push(fence.0.get_semaphore(to_iq));
                            }
                            _ => {}
                        }
                    }
                }

                let wait_sem_range = sem_start..s.sems[iq].len();

                // Assemble the list of semaphores to signal
                let sem_start = s.sems[iq].len();
                for fence in pass.update_fences.iter() {
                    let fqd: &mut FenceQueueData<_> =
                        fence.0.queue_data_write(&mut excl_data.token);
                    for (to_iq, wait_state) in fqd.wait_states.iter_mut().enumerate() {
                        match wait_state {
                            &mut Unavailable => {
                                continue;
                            }
                            &mut Semaphore { .. } => {
                                // This might happen if the application issues
                                // multiple `update_fence`s on the same-phore.
                                // We can safely ignore this.
                                continue;
                            }
                            &mut PipelineBarrier { .. } |
                            &mut Ready => {}
                        }

                        if to_iq == iq {
                            // TODO: handle render passes including some dependencies automatically
                            *wait_state = PipelineBarrier {
                                src_scope: (fence.1, fence.2),
                                dst_scope: (
                                    core::PipelineStageFlags::empty(),
                                    core::AccessTypeFlags::empty(),
                                ),
                            };
                        } else {
                            *wait_state = Semaphore { signalled_by: iq as u32 };
                            s.sems[iq].push(fence.0.get_semaphore(to_iq));
                        }
                    }
                }

                let sig_sem_range = sem_start..s.sems[iq].len();

                // Now add the pass to the current batch
                s.cbs[iq].push(pass.buffer);
                s.sem_ranges[iq].push((wait_sem_range, sig_sem_range));
            }
        }

        // Now finalize the batches...
        for i in 0..num_iqs {
            sched.resolve(i as u32, |iq_u32| { s.flush_iq(iq_u32 as usize); });
        }

        // Prepare values for `p_wait_dst_stage_mask`
        // (We use the same value for all wait ops for now)
        let mut max_num_wait_sems = 0;
        for batch in s.batches.iter() {
            for cb_i in batch.cb_range.clone() {
                let num_wait_sems = s.sem_ranges[batch.iq][cb_i].0.len();
                if num_wait_sems > max_num_wait_sems {
                    max_num_wait_sems = num_wait_sems;
                }
            }
        }
        let wait_dst_stage_masks = vec![vk::PIPELINE_STAGE_ALL_COMMANDS_BIT; max_num_wait_sems];

        // Assemble `SubmitInfo`s. Try to merge as many consecutive command
        // buffers into one `SubmitInfo` as possible.

        /// List of `SubmitInfo`s.
        let mut infos = Vec::new();

        /// List containing the starting index in `infos` and the internal queue
        /// index for every batch group.
        let mut info_is = Vec::new();

        /// List containing the last index in `info_is` for every internal queue.
        let mut last_info_is_i = [None; MAX_NUM_QUEUES];

        {
            let mut batch_i = 0;
            let mut info = vk::SubmitInfo {
                s_type: vk::StructureType::SubmitInfo,
                p_next: ptr::null(),
                wait_semaphore_count: 0,
                p_wait_semaphores: ptr::null(),
                p_wait_dst_stage_mask: wait_dst_stage_masks.as_ptr(),
                command_buffer_count: 0,
                p_command_buffers: ptr::null(),
                signal_semaphore_count: 0,
                p_signal_semaphores: ptr::null(),
            };
            while batch_i < s.batches.len() {
                let iq = s.batches[batch_i].iq;
                last_info_is_i[iq] = Some(info_is.len());
                info_is.push((infos.len(), iq));
                info.p_command_buffers = &s.cbs[iq][s.batches[batch_i].cb_range.start];
                while batch_i < s.batches.len() && s.batches[batch_i].iq == iq {
                    debug_assert!(s.batches[batch_i].cb_range.len() > 0);
                    for cb_i in s.batches[batch_i].cb_range.clone() {
                        let (ref wait_sem_is, ref sig_sem_is) = s.sem_ranges[iq][cb_i];
                        if wait_sem_is.len() > 0 {
                            if info.command_buffer_count > 0 {
                                infos.push(info.clone());
                                info.command_buffer_count = 0;
                                info.signal_semaphore_count = 0;
                                info.p_command_buffers = &s.cbs[iq][cb_i];
                            }
                            info.wait_semaphore_count = wait_sem_is.len() as u32;
                            info.p_wait_semaphores = &s.sems[iq][wait_sem_is.start];
                        }
                        info.command_buffer_count += 1;
                        if sig_sem_is.len() > 0 {
                            info.signal_semaphore_count = sig_sem_is.len() as u32;
                            info.p_signal_semaphores = &s.sems[iq][sig_sem_is.start];
                            infos.push(info.clone());
                            info.p_command_buffers = info.p_command_buffers.wrapping_offset(
                                info.command_buffer_count as
                                    isize,
                            );
                            info.command_buffer_count = 0;
                            info.wait_semaphore_count = 0;
                            info.signal_semaphore_count = 0;
                        }
                    }
					batch_i += 1;
                }
                if info.command_buffer_count > 0 {
                    infos.push(info.clone());
                    info.command_buffer_count = 0;
                    info.wait_semaphore_count = 0;
                    info.signal_semaphore_count = 0;
                }
            }
        }

        let llfence_tmp = if event.is_none() {
            Some(Arc::new(data.device_data.make_llfence(false)?))
        } else {
            None
        };
        let llfence: &Arc<LlFence<T>> = if let Some(event) = event {
            event.llfence()
        } else {
            llfence_tmp.as_ref().unwrap()
        };

        // Assemble and associate the dependency table
        LlFence::inject_deps(llfence, |mut di| {
            {
                let dep_table = if buffers.len() == 1 {
                    &mut buffers[0].data.dependency_table
                } else {
                    let dep_table = &mut excl_data.dep_table;
                    for buffer in buffers.iter_mut() {
                        dep_table.inherit(&mut buffer.data.dependency_table);
                    }
                    dep_table
                };
                di.inherit(dep_table);
            }

            for buffer in buffers.iter_mut() {
                di.insert_cbp_set(buffer.data.pools.lock_device());

                let ref mut buffer_data = *buffer.data;
                for pass in buffer_data.passes.iter_mut() {
                    for fence in pass.wait_fences.iter() {
                        let fqd: &mut FenceQueueData<_> =
                            fence.0.queue_data_write(&mut excl_data.token);
                        di.insert_semaphores(fqd.mutex.lock_device());
                    }
                    for fence in pass.update_fences.iter() {
                        let fqd: &mut FenceQueueData<_> =
                            fence.0.queue_data_write(&mut excl_data.token);
                        di.insert_semaphores(fqd.mutex.lock_device());
                    }
                }
            }
        });

        // Submit commands
        for (info_is_i, &(info_i, iq)) in info_is.iter().enumerate() {
            let info_end_i = if info_is_i + 1 >= info_is.len() {
                infos.len()
            } else {
                info_is[info_is_i + 1].0
            };

            let queue = data.queues[iq];
            let fence = if Some(info_is_i) == last_info_is_i[iq] {
                llfence.fences()[iq]
            } else {
                vk::Fence::null()
            };
            unsafe {
                device
                    .queue_submit(queue, &infos[info_i..info_end_i], fence)
                    .map_err(translate_generic_error_unwrap)?;
            }
        }

        // Even unused queues must have `vk::Fence` to ensure the total
        // chronological order of `LlFence`
        for (iq, info_is_i) in last_info_is_i.iter().enumerate() {
            if info_is_i.is_none() && iq < data.queues.len() {
                let queue = data.queues[iq];
                let fence = llfence.fences()[iq];
                unsafe {
                    device.queue_submit(queue, &[], fence).map_err(
                        translate_generic_error_unwrap,
                    )?;
                }
            }
        }

        for buffer in buffers.iter_mut() {
            buffer.data.encoder_state = EncoderState::Submitted;
        }

        llfence.mark_submitted();

        // Well, that's it! :)
        s.panicking = false;
        excl_data.lost = false;

        Ok(())
    }
}

impl<T: DeviceRef> core::CommandQueue<Backend<T>> for CommandQueue<T> {
    fn make_command_buffer(&self) -> core::Result<CommandBuffer<T>> {
        CommandBuffer::new(self.device_ref(), &self.data.device_data.cfg)
    }

    fn wait_idle(&self) {
        // Ignore errors
        let _ = self.device_ref().device().device_wait_idle();
        // FIXME: make sure or wait until all active references to `ResourceMutexDeviceRef`s are destroyed
    }

    fn submit_commands(
        &self,
        buffers: &mut [&mut CommandBuffer<T>],
        event: Option<&Event<T>>,
    ) -> core::Result<()> {
        CommandSender::submit(self, buffers, event)
    }

    fn make_fence(&self, desc: &core::FenceDescription) -> core::Result<Fence<T>> {
        Fence::with_description(self, desc)
    }
}

impl<T: DeviceRef> core::Marker for CommandQueue<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}
