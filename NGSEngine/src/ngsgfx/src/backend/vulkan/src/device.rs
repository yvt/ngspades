//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, ash};
use ash::vk;

use std::sync::Arc;

use DeviceRef;
use imp::{Backend, CommandQueue, DeviceCapabilities, DeviceConfig};

pub struct Device<T: DeviceRef> {
    data: Arc<DeviceData<T>>,
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
            device_ref,
            cap,
            cfg,
        });
        Device {
            main_queue: CommandQueue::new(&data),
            data,
        }
    }
    pub(crate) fn data(&self) -> &DeviceData<T> {
        &*self.data
    }
    pub fn device_ref(&self) -> &T {
        &self.data.device_ref
    }
    pub(crate) fn capabilities(&self) -> &DeviceCapabilities {
        &self.data.cap
    }
}
