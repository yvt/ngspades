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

impl core::Device for Device {
    type Resources = imp::Resources;
    type CommandBuffer = imp::CommandBuffer;
    type CommandQueue  = imp::CommandQueue;
    type Factory = imp::Factory;
    type DeviceCapabilities = imp::DeviceCapabilities;

    fn main_queue(&self) -> &Self::CommandQueue {
        unimplemented!()
    }
    fn factory(&self) -> &Self::Factory {
        &self.factory
    }
    fn capabilities(&self) -> &Self::DeviceCapabilities {
        &self.data.capabilities()
    }
}

impl Device {
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
