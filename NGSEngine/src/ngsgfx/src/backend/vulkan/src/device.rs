//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, ash};
use ash::vk;

use std::sync::Arc;

use DeviceRef;
use imp::{Backend, CommandQueue, DeviceCapabilities, EngineQueueMappings};

pub struct Device<T: DeviceRef> {
    data: Arc<DeviceData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for Device<T> => data
}

#[derive(Debug)]
pub(crate) struct DeviceData<T: DeviceRef> {
    device_ref: T,
    cap: DeviceCapabilities,
    queue_mappings: EngineQueueMappings,
}

impl<T: DeviceRef> core::Device<Backend<T>> for Device<T> {
    fn main_queue(&self) -> &CommandQueue<T> {
        unimplemented!()
    }
    fn factory(&self) -> &Device<T> {
        &self
    }
    fn capabilities(&self) -> &DeviceCapabilities {
        &self.data.cap
    }
}

impl<T: DeviceRef> Device<T> {
    pub fn new(
        device_ref: T,
        queue_mappings: EngineQueueMappings,
        cap: DeviceCapabilities,
    ) -> Self {
        Device {
            data: Arc::new(DeviceData{
                device_ref,
                cap,
                queue_mappings,
            }),
        }
    }
    pub(crate) fn data(&self) -> &DeviceData<T> {
        &*self.data
    }
    pub(crate) fn device_ref(&self) -> &T {
        &self.data.device_ref
    }
    pub(crate) fn capabilities(&self) -> &DeviceCapabilities {
        &self.data.cap
    }
}
