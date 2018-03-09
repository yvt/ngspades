//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdQueue` for Metal.
use std::sync::Arc;
use std::collections::HashSet;
use parking_lot::Mutex;
use tokenlock::{Token, TokenRef};
use metal::{MTLCommandBuffer, MTLCommandQueue, MTLDevice};
use block;

use base::{self, command, handles, QueueFamily};
use common::Result;
use utils::{nil_error, OCPtr};

use super::enc::CmdBufferFenceSet;
use super::buffer::CmdBuffer;
use super::fence::Fence;

/// Implementation of `CmdQueueBuilder` for Metal.
#[derive(Debug)]
pub struct CmdQueueBuilder {
    metal_device: MTLDevice,
    label: Option<String>,
}

zangfx_impl_object! { CmdQueueBuilder: command::CmdQueueBuilder, ::Debug, base::SetLabel }

unsafe impl Send for CmdQueueBuilder {}
unsafe impl Sync for CmdQueueBuilder {}

impl CmdQueueBuilder {
    /// Construct a `CmdQueueBuilder`.
    ///
    /// Ir's up to the caller to maintain the lifetime of `metal_device`.
    pub unsafe fn new(metal_device: MTLDevice) -> Self {
        Self {
            metal_device,
            label: None,
        }
    }
}

impl base::SetLabel for CmdQueueBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
    }
}

impl command::CmdQueueBuilder for CmdQueueBuilder {
    fn queue_family(&mut self, _: QueueFamily) -> &mut command::CmdQueueBuilder {
        // Ignore it since we know we only have exactly one queue family
        self
    }

    fn build(&mut self) -> Result<Box<command::CmdQueue>> {
        let metal_queue = self.metal_device.new_command_queue();
        if metal_queue.is_null() {
            Err(nil_error("MTLDevice newCommandQueue"))
        } else {
            if let Some(ref label) = self.label {
                metal_queue.set_label(label);
            }
            unsafe { Ok(Box::new(CmdQueue::from_raw(metal_queue))) }
        }
    }
}

/// Implementation of `CmdQueue` for Metal.
#[derive(Debug)]
pub struct CmdQueue {
    metal_queue: OCPtr<MTLCommandQueue>,
    device: MTLDevice,
    scheduler: Arc<Scheduler>,
}

zangfx_impl_object! { CmdQueue: command::CmdQueue, ::Debug }

unsafe impl Send for CmdQueue {}
unsafe impl Sync for CmdQueue {}

#[derive(Debug)]
pub(super) struct Scheduler {
    data: Mutex<SchedulerData>,
    token_ref: TokenRef,
}

#[derive(Debug, Default)]
pub(super) struct SchedulerData {
    /// Queue items to be processed.
    pending_items: Option<Box<Item>>,

    token: Token,
}

#[derive(Debug)]
pub(super) struct Item {
    commited: CommitedBuffer,

    /// The current index into `CmdBufferFenceSet::wait_fences` impeding the
    /// execution of this queue item.
    wait_fence_index: usize,

    /// Singly-linked list
    next: Option<Box<Item>>,
}

#[derive(Debug)]
pub(super) struct CommitedBuffer {
    pub metal_buffer: OCPtr<MTLCommandBuffer>,
    pub fence_set: CmdBufferFenceSet,
}

impl CmdQueue {
    pub unsafe fn from_raw(metal_queue: MTLCommandQueue) -> Self {
        let device = metal_queue.device();
        let scheduler_data = SchedulerData::default();
        Self {
            metal_queue: OCPtr::from_raw(metal_queue).unwrap(),
            device,
            scheduler: Arc::new(Scheduler {
                token_ref: (&scheduler_data.token).into(),
                data: Mutex::new(scheduler_data),
            }),
        }
    }
}

impl Scheduler {
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

    fn flush(this: &Arc<Self>) {
        let mut data = this.data.lock();
        let items = data.pending_items.take();
        data.process_items(items, this);
    }

    fn fence_scheduled(this: &Arc<Self>, fences: &HashSet<Fence>) {
        this.data.lock().fence_scheduled(fences, this);
    }
}

impl SchedulerData {
    /// Unblock queue items blocked by given fence(s).
    fn fence_scheduled(&mut self, fences: &HashSet<Fence>, scheduler: &Arc<Scheduler>) {
        let mut unblocked_items: Option<Box<Item>> = None;

        for fence in fences.iter() {
            let sched_data = fence.schedule_data().write(&mut self.token).unwrap();

            // Mark the fence as signaled
            sched_data.signaled = true;

            // Move its wait queue to `unblocked_items`
            let mut waiting_or_none: Option<Box<Item>> = sched_data.waiting.take();
            while let Some(mut waiting) = { waiting_or_none } {
                let next = waiting.next.take();

                // The current one (`wait_fence_index`) points `fence` and
                // therefore needn't to be checked if it is signaled again
                waiting.wait_fence_index += 1;
                waiting.next = unblocked_items;
                unblocked_items = Some(waiting);

                waiting_or_none = next;
            }
        }

        self.process_items(unblocked_items, scheduler);
    }

    /// Check a given set of items and schedule them if possible. If some of
    /// items are not schedulable, add them to blocking fences' wait queue.
    fn process_items(&mut self, items: Option<Box<Item>>, scheduler: &Arc<Scheduler>) {
        use std::mem::replace;

        let mut item_or_none = items;
        let mut schedulable_items: Option<Box<Item>> = None;

        while let Some(mut item) = { item_or_none } {
            let next = item.next.take();

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
                // The item is schedulable.
                item.next = schedulable_items;
                schedulable_items = Some(item);
            }

            item_or_none = next;
        }

        // Schedule the schedulable (not blocked by any fences) command buffers
        while let Some(mut item) = { schedulable_items } {
            {
                let ref mut commited: CommitedBuffer = item.commited;

                // Register the scheduled handler for the Metal command buffer
                let signal_fences =
                    replace(&mut commited.fence_set.signal_fences, Default::default());

                if signal_fences.len() > 0 {
                    let scheduler = Arc::clone(scheduler);
                    let block = block::ConcreteBlock::new(move |_| {
                        Scheduler::fence_scheduled(&scheduler, &signal_fences);
                    });
                    commited.metal_buffer.add_scheduled_handler(&block.copy());
                }

                // Commit the Metal command buffer
                commited.metal_buffer.commit();
            }

            schedulable_items = item.next.take();
        }
    }
}

impl command::CmdQueue for CmdQueue {
    fn new_cmd_buffer(&self) -> Result<Box<command::CmdBuffer>> {
        unsafe { CmdBuffer::new(*self.metal_queue, Arc::clone(&self.scheduler)) }
            .map(|cb| Box::new(cb) as Box<_>)
    }

    fn new_fence(&self) -> Result<handles::Fence> {
        unsafe { Fence::new(self.device, self.scheduler.token_ref.clone()) }
            .map(handles::Fence::new)
    }

    fn flush(&self) {
        Scheduler::flush(&self.scheduler);
    }
}
