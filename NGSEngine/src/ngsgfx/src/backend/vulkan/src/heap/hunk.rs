//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {DeviceRef, AshDevice};
use ash::vk;
use ash::version::DeviceV1_0;

/// Represents a single `VkDeviceMemory`.
#[derive(Debug)]
pub(crate) struct MemoryHunk<T: DeviceRef> {
    device_ref: T,
    handle: vk::DeviceMemory,
}

impl<T: DeviceRef> MemoryHunk<T> {
    pub unsafe fn from_raw(device_ref: &T, handle: vk::DeviceMemory) -> Self {
        Self {
            device_ref: device_ref.clone(),
            handle,
        }
    }

    pub fn device_ref(&self) -> &T {
        &self.device_ref
    }

    pub fn handle(&self) -> vk::DeviceMemory {
        self.handle
    }
}

impl<T: DeviceRef> Drop for MemoryHunk<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe {
            device.free_memory(self.handle, self.device_ref.allocation_callbacks());
        }
    }
}
