//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use std::sync::Arc;

use DeviceRef;
use imp::{Backend, CommandQueue, DeviceCapabilities};

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
