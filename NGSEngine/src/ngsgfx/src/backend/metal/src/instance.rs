//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;

use imp::{Environment, Device};

#[derive(Debug)]
pub struct InstanceBuilder;

impl core::InstanceBuilder<Environment> for InstanceBuilder {
    type InitializationError = !;
    type BuildError = !;

    fn new() -> Result<Self, Self::InitializationError> {
        Ok(InstanceBuilder)
    }
    fn build(&self) -> Result<Instance, Self::BuildError> {
        Ok(Instance { adapters: [Adapter] })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Adapter;

impl core::Adapter for Adapter {
    fn name(&self) -> &str {
        "System default adapter"
    }
}

#[derive(Debug)]
pub struct Instance {
    adapters: [Adapter; 1],
}

impl core::Instance<Environment> for Instance {
    type Adapter = Adapter;

    // TODO: support multiple adapters
    fn adapters(&self) -> &[Adapter] {
        &self.adapters
    }
    fn new_device_builder(&self, _: &Adapter) -> DeviceBuilder {
        DeviceBuilder
    }
}

#[derive(Debug)]
pub struct DeviceBuilder;

impl core::DeviceBuilder<Environment> for DeviceBuilder {
    type BuildError = !;
    fn build(&self) -> Result<Device, Self::BuildError> {
        let metal_device = metal::create_system_default_device();
        Ok(Device::new(metal_device))
    }
}
