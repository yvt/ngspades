//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;
use std::sync::Arc;

use {ref_hash, ref_eq, OCPtr, imp};

/// `Device` implementation for Metal.
#[derive(Debug)]
pub struct Device {
    data: Arc<DeviceData>,
    factory: imp::Factory,
}

#[derive(Debug)]
pub(crate) struct DeviceData {
    metal_device: OCPtr<metal::MTLDevice>,
    cap: imp::DeviceCapabilities,
}

impl core::Device<imp::Backend> for Device {
    fn main_queue(&self) -> &imp::CommandQueue {
        unimplemented!()
    }
    fn factory(&self) -> &imp::Factory {
        &self.factory
    }
    fn capabilities(&self) -> &imp::DeviceCapabilities {
        &self.data.capabilities()
    }
}

impl Device {
    /// Constructs a new `Device` with a supplied `MTLDevice`.
    ///
    /// `metal_device` must not be null or it will panic.
    pub fn new(metal_device: metal::MTLDevice) -> Self {
        let data = Arc::new(DeviceData{
            metal_device: OCPtr::new(metal_device).unwrap(),
            cap: imp::DeviceCapabilities::new(metal_device),
        });
        // who cares about the extra clone
        Self {
            data: data.clone(),
            factory: imp::Factory::new(data.clone()),
        }
    }

    pub(crate) fn data(&self) -> &Arc<DeviceData> {
        &self.data
    }
}

impl DeviceData {
    pub(crate) fn metal_device(&self) -> metal::MTLDevice {
        *self.metal_device
    }

    pub(crate) fn capabilities(&self) -> &imp::DeviceCapabilities {
        &self.cap
    }
}
