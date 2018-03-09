//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::thread;
use std::sync::Arc;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use ash::vk;
use ash::version::*;
use parking_lot::Mutex;

use common::Result;

use device::DeviceRef;
use utils::translate_generic_error_unwrap;

/// Maintains a set of fences, and calls a provided callback function when one
/// of them are signaled.
///
/// `T` specifies the type of callback functions.
#[derive(Debug)]
pub(super) struct Monitor<T> {
    shared: Arc<SharedData>,

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
    pub fn new(device: DeviceRef, queue: vk::Queue, num_fences: usize) -> Result<Self> {
        let (fence_sender, fence_receiver) = sync_channel(num_fences);
        let (cmd_sender, cmd_receiver) = sync_channel(num_fences + 1);

        let shared = Arc::new(SharedData { device, queue });

        // Start the monitor thread
        {
            let shared = Arc::clone(&shared);
            let fence_sender = SyncSender::clone(&fence_sender);
            thread::Builder::new()
                .spawn(move || Self::monitor_thread(shared, fence_sender, cmd_receiver))
                .unwrap();
        }

        let monitor = Self {
            shared,
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
                    .send(unsafe {
                        device.vk_device().create_fence(
                            &vk::FenceCreateInfo {
                                s_type: vk::StructureType::FenceCreateInfo,
                                p_next: ::null(),
                                flags: vk::FenceCreateFlags::empty(),
                            },
                            None,
                        )
                    }.map_err(translate_generic_error_unwrap)?)
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
        for mut cmd in cmd_receiver.iter() {
            // Wait until the fence is signaled
            let timeout = 60_000_000_000; // a minute
            loop {
                match unsafe { device.wait_for_fences(&[cmd.fence], false, timeout) } {
                    Ok(()) => break,
                    Err(vk::Result::Timeout) => Ok(()),
                    Err(e) => Err(translate_generic_error_unwrap(e)),
                }.expect("failed to wait for fences");
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

    pub fn get_fence(&self) -> MonitorFence<T> {
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
    }
}

/// This type is used to set up a fence to be waited by `Monitor` and then
/// to have its associated callback called when the fence is signaled.
pub(super) struct MonitorFence<'a, T: 'a> {
    monitor: Option<&'a Monitor<T>>,
    fence: vk::Fence,
}

impl<'a, T: 'a> MonitorFence<'a, T> {
    pub fn vk_fence(&self) -> vk::Fence {
        self.fence
    }

    /// Register a callback function for the fence.
    pub fn finish(mut self, callback: T) {
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

impl<'a, T: 'a> Drop for MonitorFence<'a, T> {
    fn drop(&mut self) {
        if let Some(monitor) = self.monitor.take() {
            monitor.fence_sender.send(self.fence).unwrap();
        }
    }
}
