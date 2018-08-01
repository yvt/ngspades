//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdQueue` for Vulkan.
use ash::version::*;
use ash::vk;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::device::DeviceRef;
use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_object};

use crate::limits::DeviceConfig;
use crate::utils::translate_generic_error_unwrap;

use super::buffer::{CmdBuffer, CmdBufferData};
use super::bufferpool::{CbPool, CbPoolItem};
use super::fence::Fence;
use super::monitor::{Monitor, MonitorHandler};
use crate::resstate;

#[derive(Debug)]
pub(crate) struct QueuePool {
    pools: Mutex<Vec<Vec<u32>>>,
}

impl QueuePool {
    crate fn new(config: &DeviceConfig) -> Self {
        let ref queues = config.queues;

        let num_qf = queues.iter().map(|&(qf, _)| qf + 1).max().unwrap_or(0);

        let mut pools = vec![Vec::new(); num_qf as usize];
        for &(qf, i) in queues.iter().rev() {
            pools[qf as usize].push(i);
        }

        Self {
            pools: Mutex::new(pools),
        }
    }

    crate fn allocate_queue(&self, queue_family: base::QueueFamily) -> u32 {
        self.pools.lock()[queue_family as usize]
            .pop()
            .expect("out of queues")
    }
}

/// Implementation of `CmdQueueBuilder` for Vulkan.
#[derive(Debug)]
pub struct CmdQueueBuilder {
    device: DeviceRef,
    queue_pool: Arc<QueuePool>,

    max_num_outstanding_batches: usize,
    queue_family: Option<base::QueueFamily>,
}

zangfx_impl_object! { CmdQueueBuilder: dyn base::CmdQueueBuilder, dyn (crate::Debug) }

impl CmdQueueBuilder {
    pub(crate) unsafe fn new(device: DeviceRef, queue_pool: Arc<QueuePool>) -> Self {
        Self {
            device,
            queue_pool,
            max_num_outstanding_batches: 8,
            queue_family: None,
        }
    }

    /// Set the maximum number of outstanding batches.
    ///
    /// Defaults to `8`.
    pub fn max_num_outstanding_batches(&mut self, v: usize) -> &mut Self {
        self.max_num_outstanding_batches = v;
        self
    }
}

impl base::CmdQueueBuilder for CmdQueueBuilder {
    fn queue_family(&mut self, v: base::QueueFamily) -> &mut dyn base::CmdQueueBuilder {
        self.queue_family = Some(v);
        self
    }

    fn build(&mut self) -> Result<base::CmdQueueRef> {
        if self.max_num_outstanding_batches < 1 {
            panic!("max_num_outstanding_batches");
        }

        let queue_family = self.queue_family.expect("queue_family");

        let index = self.queue_pool.allocate_queue(queue_family);

        let vk_device = self.device.vk_device();
        let vk_queue = unsafe { vk_device.get_device_queue(queue_family, index) };

        let num_fences = self.max_num_outstanding_batches;

        CmdQueue::new(self.device.clone(), vk_queue, queue_family, num_fences)
            .map(|x| Arc::new(x) as _)
    }
}

/// Implementation of `CmdQueue` for Vulkan.
#[derive(Debug)]
pub struct CmdQueue {
    device: DeviceRef,
    vk_queue: vk::Queue,
    queue_family_index: u32,
    cb_pool: CbPool<Box<CmdBufferData>>,
    monitor: Monitor<BatchDoneHandler>,
    scheduler: Option<Arc<Scheduler>>,
}

zangfx_impl_object! { CmdQueue: dyn base::CmdQueue, dyn (crate::Debug) }

impl Drop for CmdQueue {
    fn drop(&mut self) {
        // Drop scheduler first
        self.scheduler.take();
    }
}

impl CmdQueue {
    fn new(
        device: DeviceRef,
        vk_queue: vk::Queue,
        queue_family_index: u32,
        num_fences: usize,
    ) -> Result<Self> {
        // Initialize the resource state tracking on the newly created queue
        let (resstate_queue, resstate_cbs) = resstate::new_queue();

        // Set the default queue used during resource creation.
        device.set_default_resstate_queue_if_missing(resstate_queue.queue_id());

        let scheduler_data = SchedulerData::new(resstate_queue);
        let scheduler = Arc::new(Scheduler::new(scheduler_data));

        let cb_pool = CbPool::new(resstate_cbs.into_iter().map(|resstate_cb| {
            CmdBufferData::new(
                device.clone(),
                queue_family_index,
                scheduler.clone(),
                resstate_cb,
            ).map(Box::new)
        }))?;

        Ok(Self {
            vk_queue,
            queue_family_index,
            cb_pool,
            monitor: Monitor::new(device.clone(), vk_queue, num_fences)?,
            scheduler: Some(scheduler),
            device,
        })
    }

    fn scheduler(&self) -> &Arc<Scheduler> {
        self.scheduler.as_ref().unwrap()
    }

    pub fn vk_queue(&self) -> vk::Queue {
        self.vk_queue
    }

    /// The queue identifier for resource state tracking.
    crate fn resstate_queue_id(&self) -> resstate::QueueId {
        self.scheduler().resstate_queue_id
    }
}

impl base::CmdQueue for CmdQueue {
    fn new_cmd_buffer(&self) -> Result<base::CmdBufferRef> {
        Ok(Box::new(CmdBuffer::new(self.cb_pool.allocate())))
    }

    fn new_fence(&self) -> Result<base::FenceRef> {
        unsafe { Fence::new(self.device.clone(), self.resstate_queue_id()) }
            .map(base::FenceRef::new)
    }

    fn flush(&self) {
        self.scheduler()
            .data
            .lock()
            .flush(&self.monitor, &self.device, self.vk_queue);
    }
}

#[derive(Debug)]
crate struct Scheduler {
    data: Mutex<SchedulerData>,

    resstate_queue_id: resstate::QueueId,
}

#[derive(Debug)]
struct SchedulerData {
    /// Queue items to be processed.
    pending_items: Option<Box<Item>>,

    /// The access token used to access per-queue resource states.
    resstate_queue: resstate::Queue,
}

#[derive(Debug)]
crate struct Item {
    commited: CbPoolItem<Box<CmdBufferData>>,

    /// The current index into `FenceSet::wait_fences` impeding the
    /// execution of this queue item.
    wait_fence_index: usize,

    /// Singly-linked list
    next: Option<Box<Item>>,
}

struct ItemIter<'a>(Option<&'a Box<Item>>);

impl<'a> Iterator for ItemIter<'a> {
    type Item = &'a Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.map(|item| {
            self.0 = item.next.as_ref();
            &**item
        })
    }
}

fn for_each_item_mut<T: FnMut(&mut Item)>(item_or_none: &mut Option<Box<Item>>, mut cb: T) {
    let mut item_or_none = item_or_none.as_mut();
    while let Some(item) = { item_or_none } {
        cb(item);
        item_or_none = item.next.as_mut();
    }
}

impl Scheduler {
    fn new(data: SchedulerData) -> Self {
        Self {
            resstate_queue_id: data.resstate_queue.queue_id(),
            data: Mutex::new(data),
        }
    }

    /// Called by a command buffer's method.
    crate fn commit(&self, commited: CbPoolItem<Box<CmdBufferData>>) {
        let mut item = Box::new(Item {
            commited,
            wait_fence_index: 0,
            next: None,
        });
        let mut data = self.data.lock();
        item.next = data.pending_items.take();
        data.pending_items = Some(item);
    }
}

impl Drop for SchedulerData {
    fn drop(&mut self) {
        assert!(
            self.pending_items.is_none(),
            "there still are some pending command buffers"
        );
    }
}

impl SchedulerData {
    fn new(resstate_queue: resstate::Queue) -> Self {
        Self {
            pending_items: None,
            resstate_queue,
        }
    }
    /// Schedule and submit the queue items in `self.pending_items`. Leave
    /// unschedulable items (i.e. those which wait on fences which are not
    /// signaled by any commited items) in `self.pending_items`.
    fn flush(
        &mut self,
        monitor: &Monitor<BatchDoneHandler>,
        device: &DeviceRef,
        vk_queue: vk::Queue,
    ) {
        use crate::resstate::Resource; // for `tracked_state()`

        let mut scheduled_items = None;

        // Schedule as many items as possible.
        {
            let mut scheduled_item_tail = &mut scheduled_items;

            while let Some(mut item) = self.pending_items.take() {
                self.pending_items = item.next.take();

                // Check fences the queue item waits on
                let mut i = item.wait_fence_index;
                while let Some(&fence_i) = item.commited.fence_set.wait_fences.get(i) {
                    let entry = item.commited.ref_table.fences.get_by_index(fence_i);
                    let fence = entry.resource;

                    let sched_data = fence.tracked_state().latest_mut(&mut self.resstate_queue);

                    if sched_data.signaled {
                        // The fence is signaled by one of the command buffers
                        // that are already scheduled
                        i += 1;
                    } else {
                        break;
                    }
                }
                item.wait_fence_index = i;

                if item.wait_fence_index < item.commited.fence_set.wait_fences.len() {
                    // The scheduling of this item is blocked by one of its waiting
                    // fence.

                    // First we need to break the borrowing chain (from `item` to
                    // the fence) because we are moving the `item` to the fence's
                    // wait queue. It is definitely safe to move `item` around while
                    // keeping a reference to `fence`.
                    let fence: &Fence = unsafe {
                        let fence_i = item.commited.fence_set.wait_fences[item.wait_fence_index];
                        let entry = item.commited.ref_table.fences.get_by_index(fence_i);
                        let fence = entry.resource;
                        &*(fence as *const _)
                    };

                    // Insert `item` to the fence's wait queue.
                    let sched_data = fence.tracked_state().latest_mut(&mut self.resstate_queue);
                    item.next = sched_data.waiting.take();
                    sched_data.waiting = Some(item);
                } else {
                    // The item is schedulable. Schedule the item, and unblock
                    // other items waiting on a fence that was just signaled by
                    // this item.
                    let ref mut commited = *item.commited;
                    for &fence_i in commited.fence_set.signal_fences.iter() {
                        let entry = commited.ref_table.fences.get_by_index(fence_i);
                        let fence = entry.resource;
                        let sched_data = fence.tracked_state().latest_mut(&mut self.resstate_queue);

                        // Mark the fence as signaled
                        sched_data.signaled = true;

                        // Move its wait queue to `pending_items`
                        let mut waiting_or_none: Option<Box<Item>> = sched_data.waiting.take();
                        while let Some(mut waiting) = { waiting_or_none } {
                            let next = waiting.next.take();

                            // The current one (`wait_fence_index`) points `fence` and
                            // therefore needn't to be checked if it is signaled again
                            waiting.wait_fence_index += 1;
                            waiting.next = self.pending_items.take();
                            self.pending_items = Some(waiting);

                            waiting_or_none = next;
                        }
                    }

                    // Schedule this item.
                    *scheduled_item_tail = Some(item);
                    scheduled_item_tail = &mut { scheduled_item_tail }.as_mut().unwrap().next;
                }
            }
        }

        if scheduled_items.is_none() {
            return;
        }

        // Create submission batches
        let fence = monitor.get_fence();
        let vk_device = device.vk_device();

        let mut num_cmd_buffers = 0;
        let mut num_wait_semaphores = 0;
        let mut num_signal_semaphores = 0;

        for item in ItemIter(scheduled_items.as_ref()) {
            let ref commited = item.commited;
            // TODO: Take patched command buffers into account
            num_cmd_buffers += commited.passes.len();
            num_wait_semaphores += commited.wait_semaphores.len();
            num_signal_semaphores += commited.signal_semaphores.len();
        }

        // Hold the objects from all batches
        let mut vk_cmd_buffers = Vec::with_capacity(num_cmd_buffers);
        let mut vk_wait_sems = Vec::with_capacity(num_wait_semaphores);
        let mut vk_wait_sem_stages = Vec::with_capacity(num_wait_semaphores);
        let mut vk_signal_sems = Vec::with_capacity(num_signal_semaphores);

        let mut vk_submit_infos = Vec::with_capacity(num_cmd_buffers);

        // The starting addresses for objects in the current batch
        fn vec_end_ptr<T>(v: &[T]) -> *const T {
            v.as_ptr().wrapping_offset(v.len() as isize)
        }
        let mut p_cmd_buffers = vec_end_ptr(&vk_cmd_buffers);
        let mut p_wait_sems = vec_end_ptr(&vk_wait_sems);
        let mut p_wait_sem_stages = vec_end_ptr(&vk_wait_sem_stages);
        let mut p_signal_sems = vec_end_ptr(&vk_signal_sems);

        // The state of the current batch
        let mut terminate_current_batch = false;
        let mut cur_num_cmd_buffers = 0;
        let mut cur_num_wait_sems = 0;
        let mut cur_num_signal_sems = 0;

        macro_rules! flush {
            () => {
                if cur_num_cmd_buffers > 0 {
                    let vk_submit_info = vk::SubmitInfo {
                        s_type: vk::StructureType::SubmitInfo,
                        p_next: ::null(),
                        wait_semaphore_count: cur_num_wait_sems as u32,
                        p_wait_semaphores: p_wait_sems,
                        p_wait_dst_stage_mask: p_wait_sem_stages,
                        command_buffer_count: cur_num_cmd_buffers as u32,
                        p_command_buffers: p_cmd_buffers,
                        signal_semaphore_count: cur_num_signal_sems as u32,
                        p_signal_semaphores: p_signal_sems,
                    };
                    vk_submit_infos.push(vk_submit_info);
                }
            };
        }

        for item in ItemIter(scheduled_items.as_ref()) {
            let ref commited = item.commited;

            if commited.wait_semaphores.len() > 0 {
                terminate_current_batch = true;
            }

            if terminate_current_batch && cur_num_cmd_buffers > 0 {
                flush!();

                p_cmd_buffers = vec_end_ptr(&vk_cmd_buffers);
                p_wait_sems = vec_end_ptr(&vk_wait_sems);
                p_wait_sem_stages = vec_end_ptr(&vk_wait_sem_stages);
                p_signal_sems = vec_end_ptr(&vk_signal_sems);
                cur_num_cmd_buffers = 0;
                cur_num_wait_sems = 0;
                cur_num_signal_sems = 0;
            }

            terminate_current_batch = false;

            for pass in commited.passes.iter() {
                vk_cmd_buffers.push(pass.vk_cmd_buffer);
                cur_num_cmd_buffers += 1;
            }

            let ref wait_sems = commited.wait_semaphores;
            vk_wait_sems.extend(wait_sems.iter().map(|&(ref sem, _)| sem.vk_semaphore()));
            vk_wait_sem_stages.extend(wait_sems.iter().map(|&(_, stages)| stages));
            cur_num_wait_sems += wait_sems.len();

            let ref signal_sems = commited.signal_semaphores;
            vk_signal_sems.extend(signal_sems.iter().map(|sem| sem.vk_semaphore()));
            cur_num_signal_sems += signal_sems.len();

            if commited.signal_semaphores.len() > 0 {
                terminate_current_batch = true;
            }
        }

        if cur_num_cmd_buffers > 0 || cur_num_signal_sems > 0 || cur_num_wait_sems > 0 {
            flush!();
        }

        let done_handler = BatchDoneHandler { scheduled_items };

        let result =
            unsafe { vk_device.queue_submit(vk_queue, &vk_submit_infos, fence.vk_fence()) };

        if let Err(err) = result {
            done_handler.finish(|| Err(translate_generic_error_unwrap(err)));
            return;
        }

        // Call `BatchDoneHandler::on_fence_signaled` when the batch is complete
        fence.finish(done_handler);
    }
}

#[derive(Debug)]
pub(super) struct BatchDoneHandler {
    scheduled_items: Option<Box<Item>>,
}

impl BatchDoneHandler {
    fn finish(self, mut result: impl FnMut() -> Result<()>) {
        let mut scheduled_items = self.scheduled_items;

        // Release objects first (because completion callbacks might tear
        // down the device)
        for_each_item_mut(&mut scheduled_items, |item| {
            let ref mut commited = item.commited;
            commited.reset();
        });

        // Call the completion callbacks
        while let Some(mut item) = { scheduled_items } {
            item.commited.completion_callbacks.on_complete(&mut result);
            scheduled_items = item.next;
        }
    }
}

impl MonitorHandler for BatchDoneHandler {
    fn on_fence_signaled(self) {
        self.finish(|| Ok(()))
    }
}
