//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::version::*;
use ash::vk;
use parking_lot::Mutex;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::thread;

use zangfx_base::Result;

use crate::device::DeviceRef;
use crate::utils::translate_generic_error_unwrap;

/// Maintains a set of fences, and calls a provided callback function when one
/// of them are signaled.
///
/// `T` specifies the type of callback functions.
#[derive(Debug)]
pub(super) struct Monitor<T> {
    shared: Arc<SharedData>,
    join_handle: Option<thread::JoinHandle<()>>,
    fence_receiver: Mutex<Receiver<vk::Fence>>,
    fence_sender: SyncSender<vk::Fence>,
    cmd_sender: Option<SyncSender<Cmd<T>>>,
}

pub(super) trait MonitorHandler: 'static + Send {
    fn on_fence_signaled(self);
}

struct Cmd<T> {
    fence: vk::Fence,
    callback: T,
}

#[derive(Debug)]
struct SharedData {
    device: DeviceRef,
    queue: vk::Queue,
}

impl<T> Monitor<T>
where
    T: MonitorHandler,
{
    crate fn new(device: DeviceRef, queue: vk::Queue, num_fences: usize) -> Result<Self> {
        let (fence_sender, fence_receiver) = sync_channel(num_fences);
        let (cmd_sender, cmd_receiver) = sync_channel(num_fences + 1);

        let shared = Arc::new(SharedData { device, queue });

        // Start the monitor thread
        let join_handle = {
            let shared = Arc::clone(&shared);
            let fence_sender = SyncSender::clone(&fence_sender);
            thread::Builder::new()
                .spawn(move || Self::monitor_thread(shared, fence_sender, cmd_receiver))
                .unwrap()
        };

        let monitor = Self {
            shared,
            join_handle: Some(join_handle),
            fence_receiver: Mutex::new(fence_receiver),
            fence_sender,
            cmd_sender: Some(cmd_sender),
        };

        // Create fences (this is done after constructing `Monitor` just in case
        // the creation of one of them fails)
        {
            let ref sender = monitor.fence_sender;
            for _ in 0..num_fences {
                sender
                    .send(
                        unsafe {
                            let ref vk_device = monitor.shared.device.vk_device();
                            vk_device.create_fence(
                                &vk::FenceCreateInfo {
                                    s_type: vk::StructureType::FENCE_CREATE_INFO,
                                    p_next: crate::null(),
                                    flags: vk::FenceCreateFlags::empty(),
                                },
                                None,
                            )
                        }
                        .map_err(translate_generic_error_unwrap)?,
                    )
                    .unwrap();
            }
        }

        Ok(monitor)
    }

    fn monitor_thread(
        shared: Arc<SharedData>,
        fence_sender: SyncSender<vk::Fence>,
        cmd_receiver: Receiver<Cmd<T>>,
    ) {
        let device = shared.device.vk_device();
        for cmd in cmd_receiver.iter() {
            // Wait until the fence is signaled
            let timeout = 60_000_000_000; // a minute
            loop {
                match unsafe { device.wait_for_fences(&[cmd.fence], false, timeout) } {
                    Ok(()) => break,
                    Err(vk::Result::TIMEOUT) => Ok(()),
                    Err(e) => Err(translate_generic_error_unwrap(e)),
                }
                .expect("failed to wait for fences");
            }

            // This fence is available for next use
            unsafe { device.reset_fences(&[cmd.fence]) }
                .map_err(translate_generic_error_unwrap)
                .unwrap();
            fence_sender.send(cmd.fence).unwrap();

            // Call the callback for the fence (Note that this callback
            // function might drop `Monitor`)
            cmd.callback.on_fence_signaled();
        }
    }

    crate fn get_fence(&self) -> MonitorFence<'_, T> {
        let fence = self.fence_receiver.lock().recv().unwrap();
        MonitorFence {
            monitor: Some(self),
            fence,
        }
    }
}

impl<T> Drop for Monitor<T> {
    fn drop(&mut self) {
        // Hang up the channel (which causes the monitor thread to quit)
        self.cmd_sender = None;

        // Drop fences. We can't use `Receiver::iter()` here because it'll
        // dead lock if this method was called inside a callback function
        let device = self.shared.device.vk_device();
        for fence in self.fence_receiver.get_mut().try_iter() {
            unsafe { device.destroy_fence(fence, None) };
        }

        // Wait until the monitor thread exits -- otherwise a race condition
        // between `VkCmdQueue`'s destructor and `free_command_buffers`
        // called by `on_fence_signaled` might occur
        let join_handle = self.join_handle.take().unwrap();
        if thread::current().id() != join_handle.thread().id() {
            join_handle.join().unwrap();
        }
    }
}

/// This type is used to set up a fence to be waited by `Monitor` and then
/// to have its associated callback called when the fence is signaled.
pub(super) struct MonitorFence<'a, T> {
    monitor: Option<&'a Monitor<T>>,
    fence: vk::Fence,
}

impl<'a, T> MonitorFence<'a, T> {
    crate fn vk_fence(&self) -> vk::Fence {
        self.fence
    }

    /// Register a callback function for the fence.
    crate fn finish(mut self, callback: T) {
        let monitor = self.monitor.take().unwrap();
        monitor
            .cmd_sender
            .as_ref()
            .unwrap()
            .send(Cmd {
                fence: self.fence,
                callback,
            })
            .unwrap();
    }
}

impl<'a, T> Drop for MonitorFence<'a, T> {
    fn drop(&mut self) {
        if let Some(monitor) = self.monitor.take() {
            monitor.fence_sender.send(self.fence).unwrap();
        }
    }
}
