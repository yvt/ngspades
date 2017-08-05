//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use std::sync::Arc;

use DeviceRef;
use imp::{Backend, CommandQueue, DeviceCapabilities, DeviceConfig, LlFenceFactory, LlFence, Event};

pub struct Device<T: DeviceRef> {
    pub(crate) data: Arc<DeviceData<T>>,
    main_queue: CommandQueue<T>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for Device<T> => data
}

#[derive(Debug)]
pub(crate) struct DeviceData<T: DeviceRef> {
    pub(crate) device_ref: T,
    pub(crate) cap: DeviceCapabilities,
    pub(crate) cfg: DeviceConfig,
    pub(crate) llfence_factory: LlFenceFactory<T>,
}

impl<T: DeviceRef> core::Device<Backend<T>> for Device<T> {
    fn main_queue(&self) -> &CommandQueue<T> {
        &self.main_queue
    }
    fn factory(&self) -> &Device<T> {
        &self
    }
    fn capabilities(&self) -> &DeviceCapabilities {
        &self.data.cap
    }
}

impl<T: DeviceRef> Device<T> {
    pub fn new(device_ref: T, cfg: DeviceConfig, cap: DeviceCapabilities) -> Self {
        let data = Arc::new(DeviceData {
            llfence_factory: LlFenceFactory::new(device_ref.clone()),
            device_ref,
            cap,
            cfg,
        });
        Device {
            main_queue: CommandQueue::new(&data),
            data,
        }
    }
    pub fn device_ref(&self) -> &T {
        &self.data.device_ref
    }
    pub(crate) fn capabilities(&self) -> &DeviceCapabilities {
        &self.data.cap
    }
}

impl<T: DeviceRef> DeviceData<T> {
    pub fn make_llfence(&self, signaled: bool) -> core::Result<LlFence<T>> {
        self.llfence_factory.build(self.cfg.queues.len(), signaled)
    }

    pub fn make_event(&self, desc: &core::EventDescription) -> core::Result<Event<T>> {
        Ok(Event::new(Arc::new(self.make_llfence(desc.signaled)?)))
    }
}
