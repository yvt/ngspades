//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdQueue` for Vulkan.
use std::sync::Arc;
use ash::vk;
use ash::version::*;
use parking_lot::Mutex;
use tokenlock::{Token, TokenRef};

use base;
use common::{Error, ErrorKind, Result};
use device::DeviceRef;

use utils::translate_generic_error_unwrap;
use limits::DeviceConfig;

use super::monitor::{Monitor, MonitorHandler};
use super::fence::Fence;
use super::enc::{FenceSet, RefTable};
use super::buffer::BufferCompleteCallback;
use super::bufferpool::VkCmdBufferPoolItem;
use super::pool::CmdPool;
use super::semaphore::Semaphore;

#[derive(Debug)]
pub(crate) struct QueuePool {
    pools: Mutex<Vec<Vec<u32>>>,
}

impl QueuePool {
    pub fn new(config: &DeviceConfig) -> Self {
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

    pub fn allocate_queue(&self, queue_family: base::QueueFamily) -> u32 {
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

    max_num_outstanding_cmd_buffers: usize,
    max_num_outstanding_batches: usize,
    queue_family: Option<base::QueueFamily>,
}

zangfx_impl_object! { CmdQueueBuilder: base::CmdQueueBuilder, ::Debug }

impl CmdQueueBuilder {
    pub(crate) unsafe fn new(device: DeviceRef, queue_pool: Arc<QueuePool>) -> Self {
        Self {
            device,
            queue_pool,
            max_num_outstanding_cmd_buffers: 64,
            max_num_outstanding_batches: 8,
            queue_family: None,
        }
    }

    /// Set the maximum number of outstanding command buffers per command pool.
    ///
    /// Defaults to `64`.
    pub fn max_num_outstanding_cmd_buffers(&mut self, v: usize) -> &mut Self {
        self.max_num_outstanding_cmd_buffers = v;
        self
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
    fn queue_family(&mut self, v: base::QueueFamily) -> &mut base::CmdQueueBuilder {
        self.queue_family = Some(v);
        self
    }

    fn build(&mut self) -> Result<Box<base::CmdQueue>> {
        if self.max_num_outstanding_cmd_buffers < 1 {
            return Err(Error::with_detail(
                ErrorKind::InvalidUsage,
                "max_num_outstanding_cmd_buffers",
            ));
        }

        if self.max_num_outstanding_batches < 1 {
            return Err(Error::with_detail(
                ErrorKind::InvalidUsage,
                "max_num_outstanding_batches",
            ));
        }

        let queue_family = self.queue_family
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "queue_family"))?;

        let index = self.queue_pool.allocate_queue(queue_family);

        let vk_device = self.device.vk_device();
        let vk_queue = unsafe { vk_device.get_device_queue(queue_family, index) };

        let num_fences = self.max_num_outstanding_batches;
        let num_cbs = self.max_num_outstanding_cmd_buffers;

        CmdQueue::new(self.device, vk_queue, queue_family, num_fences, num_cbs)
            .map(|x| Box::new(x) as _)
    }
}

/// Implementation of `CmdQueue` for Vulkan.
#[derive(Debug)]
pub struct CmdQueue {
    device: DeviceRef,
    vk_queue: vk::Queue,
    queue_family_index: u32,
    num_cbs: usize,
    monitor: Monitor<BatchDoneHandler>,
    scheduler: Option<Arc<Scheduler>>,
}

zangfx_impl_object! { CmdQueue: base::CmdQueue, ::Debug }

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
        num_cbs: usize,
    ) -> Result<Self> {
        let scheduler_data = SchedulerData::default();

        Ok(Self {
            device,
            vk_queue,
            queue_family_index,
            num_cbs,
            monitor: Monitor::new(device, vk_queue, num_fences)?,
            scheduler: Some(Arc::new(Scheduler {
                token_ref: (&scheduler_data.token).into(),
                data: Mutex::new(scheduler_data),
            })),
        })
    }

    fn scheduler(&self) -> &Arc<Scheduler> {
        self.scheduler.as_ref().unwrap()
    }
}

impl base::CmdQueue for CmdQueue {
    fn new_cmd_pool(&self) -> Result<Box<base::CmdPool>> {
        CmdPool::new(
            self.device,
            Arc::clone(&self.scheduler()),
            self.queue_family_index,
            self.num_cbs,
        ).map(|x| Box::new(x) as _)
    }

    fn new_fence(&self) -> Result<base::Fence> {
        unsafe { Fence::new(self.device, self.scheduler().token_ref.clone()) }.map(base::Fence::new)
    }

    fn flush(&self) {
        self.scheduler()
            .data
            .lock()
            .flush(&self.monitor, self.device, self.vk_queue);
    }
}

#[derive(Debug)]
pub(super) struct Scheduler {
    data: Mutex<SchedulerData>,

    /// Used to *construct* scheduler-specific data in `Fence`s.
    token_ref: TokenRef,
}

#[derive(Debug, Default)]
struct SchedulerData {
    /// Queue items to be processed.
    pending_items: Option<Box<Item>>,

    /// Used to *access* scheduler-specific data in `Fence`s.
    token: Token,
}

#[derive(Debug)]
pub(super) struct Item {
    commited: CommitedBuffer,

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

#[derive(Debug)]
pub(super) struct CommitedBuffer {
    pub fence_set: FenceSet,
    pub ref_table: Option<RefTable>,
    pub vk_cmd_buffer_pool_item: Option<VkCmdBufferPoolItem>,
    pub completion_handler: BufferCompleteCallback,
    pub wait_semaphores: Vec<(Semaphore, vk::PipelineStageFlags)>,
    pub signal_semaphores: Vec<Semaphore>,
}

impl Scheduler {
    /// Called by a command buffer's method.
    pub fn commit(&self, commited_buffer: CommitedBuffer) {
        let mut item = Box::new(Item {
            commited: commited_buffer,
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
    /// Schedule and submit the queue items in `self.pending_items`. Leave
    /// unschedulable items (i.e. those which wait on fences which are not
    /// signaled by any commited items) in `self.pending_items`.
    fn flush(
        &mut self,
        monitor: &Monitor<BatchDoneHandler>,
        device: DeviceRef,
        vk_queue: vk::Queue,
    ) {
        let mut scheduled_items = None;

        // Schedule as many items as possible.
        {
            let mut scheduled_item_tail = &mut scheduled_items;

            while let Some(mut item) = self.pending_items.take() {
                self.pending_items = item.next.take();

                // Check fences the queue item waits on
                let mut i = item.wait_fence_index;
                while let Some(fence) = item.commited.fence_set.wait_fences.get(i) {
                    let sched_data = fence.schedule_data().write(&mut self.token).unwrap();
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
                        let ref fence = item.commited.fence_set.wait_fences[item.wait_fence_index];
                        &*(fence as *const _)
                    };

                    // Insert `item` to the fence's wait queue.
                    let sched_data = fence.schedule_data().write(&mut self.token).unwrap();
                    item.next = sched_data.waiting.take();
                    sched_data.waiting = Some(item);
                } else {
                    // The item is schedulable. Schedule the item, and unblock
                    // other items waiting on a fence that was just signaled by
                    // this item.
                    for fence in item.commited.fence_set.signal_fences.iter() {
                        let sched_data = fence.schedule_data().write(&mut self.token).unwrap();

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
            num_cmd_buffers += 1;
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
            () => (if cur_num_cmd_buffers > 0 {
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
            })
        }

        for item in ItemIter(scheduled_items.as_ref()) {
            let ref commited: CommitedBuffer = item.commited;

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

            let vk_cmd_buffer = commited
                .vk_cmd_buffer_pool_item
                .as_ref()
                .unwrap()
                .vk_cmd_buffer();
            vk_cmd_buffers.push(vk_cmd_buffer);
            cur_num_cmd_buffers += 1;

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

        if cur_num_cmd_buffers > 0 {
            flush!();
        }

        // TODO: safe handling of error
        unsafe { vk_device.queue_submit(vk_queue, &vk_submit_infos, fence.vk_fence()) }
            .map_err(translate_generic_error_unwrap)
            .unwrap();

        // Call `BatchDoneHandler::on_fence_signaled` when the batch is complete
        fence.finish(BatchDoneHandler { scheduled_items });
    }
}

#[derive(Debug)]
pub(super) struct BatchDoneHandler {
    scheduled_items: Option<Box<Item>>,
}

impl MonitorHandler for BatchDoneHandler {
    fn on_fence_signaled(self) {
        let mut scheduled_items = self.scheduled_items;

        // Release objects first (because completion callbacks might tear
        // down the device)
        for_each_item_mut(&mut scheduled_items, |item| {
            let ref mut commited: CommitedBuffer = item.commited;
            commited.ref_table = None;
            commited.vk_cmd_buffer_pool_item = None;
            commited.wait_semaphores.clear();
            commited.signal_semaphores.clear();
        });

        // Call the completion callbacks
        while let Some(mut item) = { scheduled_items } {
            item.commited.completion_handler.on_complete();
            scheduled_items = item.next;
        }
    }
}
