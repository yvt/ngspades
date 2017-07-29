//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use std::{ptr, mem};
use std::sync::{Arc, Mutex};
use ash::vk;
use ash::version::DeviceV1_0;

use {RefEqArc, DeviceRef, AshDevice, translate_generic_error_unwrap};
use command::mutex::{ResourceMutex, ResourceMutexRef};
use imp::{MemoryHunk, LlFence};

pub(crate) struct UnassociatedBuffer<'a, T: DeviceRef> {
    device_ref: &'a T,
    handle: vk::Buffer,
}

impl<'a, T: DeviceRef> UnassociatedBuffer<'a, T> {
    pub(crate) fn new(device_ref: &'a T, desc: &core::BufferDescription) -> core::Result<Self> {
        let mut usage = vk::BufferUsageFlags::empty();
        if desc.usage.contains(core::BufferUsage::TransferSource) {
            usage |= vk::BUFFER_USAGE_TRANSFER_SRC_BIT;
        }
        if desc.usage.contains(core::BufferUsage::TransferDestination) {
            usage |= vk::BUFFER_USAGE_TRANSFER_DST_BIT;
        }
        if desc.usage.contains(core::BufferUsage::UniformBuffer) {
            usage |= vk::BUFFER_USAGE_UNIFORM_BUFFER_BIT;
        }
        if desc.usage.contains(core::BufferUsage::StorageBuffer) {
            usage |= vk::BUFFER_USAGE_STORAGE_BUFFER_BIT;
        }
        if desc.usage.contains(core::BufferUsage::IndexBuffer) {
            usage |= vk::BUFFER_USAGE_INDEX_BUFFER_BIT;
        }
        if desc.usage.contains(core::BufferUsage::IndirectBuffer) {
            usage |= vk::BUFFER_USAGE_INDIRECT_BUFFER_BIT;
        }

        let info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BufferCreateInfo,
            p_next: ptr::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: desc.size,
            usage,
            sharing_mode: vk::SharingMode::Exclusive,
            queue_family_index_count: 0, // ignored for `SharingMode::Exclusive`
            p_queue_family_indices: ptr::null(),
        };

        let device: &AshDevice = device_ref.device();
        let handle = unsafe { device.create_buffer(&info, device_ref.allocation_callbacks()) }
            .map_err(translate_generic_error_unwrap)?;

        Ok(UnassociatedBuffer { device_ref, handle })
    }

    pub(crate) fn memory_requirements(&self) -> vk::MemoryRequirements {
        let device: &AshDevice = self.device_ref.device();
        device.get_buffer_memory_requirements(self.handle)
    }

    pub(crate) fn into_raw(mut self) -> vk::Buffer {
        mem::replace(&mut self.handle, vk::Buffer::null())
    }

    pub(crate) fn associate(
        self,
        hunk: Arc<MemoryHunk<T>>,
        offset: vk::DeviceSize,
    ) -> core::Result<Buffer<T>> {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.bind_buffer_memory(self.handle, hunk.handle(), offset) }
            .map_err(translate_generic_error_unwrap)?;

        let bld = BufferLockData{
            hunk,
            handle: self.into_raw(),
        };
        Ok(Buffer {
            data: RefEqArc::new(BufferData {
                handle: bld.handle,
                mutex: Mutex::new(ResourceMutex::new(bld)),
            }),
        })
    }
}

impl<'a, T: DeviceRef> Drop for UnassociatedBuffer<'a, T> {
    fn drop(&mut self) {
        if self.handle != vk::Buffer::null() {
            let device: &AshDevice = self.device_ref.device();
            unsafe { device.destroy_buffer(self.handle, self.device_ref.allocation_callbacks()) };
        }
    }
}

pub struct Buffer<T: DeviceRef> {
    data: RefEqArc<BufferData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Buffer<T> => data
}

#[derive(Debug)]
struct BufferData<T: DeviceRef> {
    /// Copy of `BufferLockData::handle`. (Do not destroy!)
    handle: vk::Buffer,
    mutex: Mutex<ResourceMutex<LlFence<T>, BufferLockData<T>>>,
}

#[derive(Debug)]
pub(crate) struct BufferLockData<T: DeviceRef> {
    hunk: Arc<MemoryHunk<T>>,
    handle: vk::Buffer,
}

impl<T: DeviceRef> core::Buffer for Buffer<T> {}

impl<T: DeviceRef> core::Marker for Buffer<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> Drop for BufferLockData<T> {
    fn drop(&mut self) {
        let device_ref = self.hunk.device_ref();
        let device: &AshDevice = device_ref.device();
        unsafe { device.destroy_buffer(self.handle, device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> Buffer<T> {
    pub fn handle(&self) -> vk::Buffer {
        self.data.handle
    }

    pub(crate) fn lock_device(&self) -> ResourceMutexRef<LlFence<T>, BufferLockData<T>> {
        self.data.mutex.lock().unwrap().lock_device().clone()
    }
}
