//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Buffer` for Vulkan.
use ash::version::*;
use ash::{prelude::VkResult, vk};
use std::sync::Arc;

use crate::device::DeviceRef;
use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{zangfx_impl_handle, zangfx_impl_object};

use crate::utils::{
    queue_id_from_queue, translate_generic_error_unwrap, translate_memory_req, QueueIdBuilder,
};
use crate::{heap, resstate};

/// Implementation of `BufferBuilder` for Vulkan.
#[derive(Debug)]
pub struct BufferBuilder {
    device: DeviceRef,
    queue_id: QueueIdBuilder,
    size: Option<base::DeviceSize>,
    usage: base::BufferUsageFlags,
}

zangfx_impl_object! { BufferBuilder: dyn base::BufferBuilder, dyn (crate::Debug) }

impl BufferBuilder {
    crate fn new(device: DeviceRef) -> Self {
        Self {
            device,
            queue_id: QueueIdBuilder::new(),
            size: None,
            usage: base::BufferUsageFlags::default(),
        }
    }
}

impl base::BufferBuilder for BufferBuilder {
    fn queue(&mut self, queue: &base::CmdQueueRef) -> &mut dyn base::BufferBuilder {
        self.queue_id.set(queue);
        self
    }

    fn size(&mut self, v: base::DeviceSize) -> &mut dyn base::BufferBuilder {
        self.size = Some(v);
        self
    }

    fn usage(&mut self, v: base::BufferUsageFlags) -> &mut dyn base::BufferBuilder {
        self.usage = v;
        self
    }

    fn build(&mut self) -> Result<base::BufferRef> {
        let size = self.size.expect("size");

        let mut usage = vk::BufferUsageFlags::empty();
        if self.usage.contains(base::BufferUsageFlags::VERTEX) {
            usage |= vk::BufferUsageFlags::VERTEX_BUFFER;
        }
        if self.usage.contains(base::BufferUsageFlags::COPY_READ) {
            usage |= vk::BufferUsageFlags::TRANSFER_SRC;
        }
        if self.usage.contains(base::BufferUsageFlags::COPY_WRITE) {
            usage |= vk::BufferUsageFlags::TRANSFER_DST;
        }
        if self.usage.contains(base::BufferUsageFlags::UNIFORM) {
            usage |= vk::BufferUsageFlags::UNIFORM_BUFFER;
        }
        if self.usage.contains(base::BufferUsageFlags::STORAGE) {
            usage |= vk::BufferUsageFlags::STORAGE_BUFFER;
        }
        if self.usage.contains(base::BufferUsageFlags::INDEX) {
            usage |= vk::BufferUsageFlags::INDEX_BUFFER;
        }
        if self.usage.contains(base::BufferUsageFlags::INDIRECT_DRAW) {
            usage |= vk::BufferUsageFlags::INDIRECT_BUFFER;
        }

        let info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            p_next: crate::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0, // ignored for `SharingMode::EXCLUSIVE`
            p_queue_family_indices: crate::null(),
        };

        let device = self.device.clone();
        let vk_buffer = unsafe {
            let vk_device = device.vk_device();
            vk_device.create_buffer(&info, None)
        }
        .map_err(translate_generic_error_unwrap)?;

        let vulkan_buffer = Arc::new(VulkanBuffer {
            device,
            vk_buffer,
            len: size,
            binding_info: heap::HeapBindingInfo::new(),
        });

        let queue_id = self.queue_id.get(&vulkan_buffer.device);
        let tracked_state = Arc::new(resstate::TrackedState::new(queue_id, ()));

        Ok(Buffer {
            vulkan_buffer,
            tracked_state,
        }
        .into())
    }
}

/// Implementation of `Buffer` for Vulkan.
#[derive(Debug, Clone)]
pub struct Buffer {
    vulkan_buffer: Arc<VulkanBuffer>,

    /// The container for the tracked state of an image on a particular queue.
    tracked_state: Arc<resstate::TrackedState<BufferState>>,
}

zangfx_impl_handle! { Buffer, base::BufferRef }

#[derive(Debug)]
struct VulkanBuffer {
    device: DeviceRef,
    vk_buffer: vk::Buffer,
    len: base::DeviceSize,
    binding_info: heap::HeapBindingInfo,
}

type BufferState = ();

impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        unsafe {
            let vk_device = self.device.vk_device();
            vk_device.destroy_buffer(self.vk_buffer, None);
        }
    }
}

impl Buffer {
    // TODO: `Buffer::from_raw`
    /* pub unsafe fn from_raw(vk_buffer: vk::Buffer) -> Self {
        Self { vk_buffer }
    } */

    pub fn vk_buffer(&self) -> vk::Buffer {
        self.vulkan_buffer.vk_buffer
    }
}

impl VulkanBuffer {
    fn memory_req(&self) -> base::MemoryReq {
        let vk_device = self.device.vk_device();
        translate_memory_req(&unsafe { vk_device.get_buffer_memory_requirements(self.vk_buffer) })
    }
}

impl resstate::Resource for Buffer {
    type State = BufferState;

    fn tracked_state(&self) -> &resstate::TrackedState<Self::State> {
        &self.tracked_state
    }
}

unsafe impl base::Buffer for Buffer {
    fn as_ptr(&self) -> *mut u8 {
        self.vulkan_buffer.binding_info.as_ptr()
    }

    fn len(&self) -> base::DeviceSize {
        self.vulkan_buffer.len
    }

    fn make_proxy(&self, queue: &base::CmdQueueRef) -> base::BufferRef {
        let queue_id = queue_id_from_queue(queue);

        let vulkan_buffer = self.vulkan_buffer.clone();

        // Create a fresh tracked state for the target queue
        let tracked_state = Arc::new(resstate::TrackedState::new(queue_id, ()));

        Buffer {
            vulkan_buffer,
            tracked_state,
        }
        .into()
    }

    fn get_memory_req(&self) -> Result<base::MemoryReq> {
        Ok(self.vulkan_buffer.memory_req())
    }
}

impl heap::Bindable for Buffer {
    fn memory_req(&self) -> base::MemoryReq {
        self.vulkan_buffer.memory_req()
    }

    fn binding_info(&self) -> &heap::HeapBindingInfo {
        &self.vulkan_buffer.binding_info
    }

    unsafe fn bind(
        &self,
        vk_device_memory: vk::DeviceMemory,
        offset: vk::DeviceSize,
    ) -> VkResult<()> {
        let vk_device = self.vulkan_buffer.device.vk_device();
        vk_device.bind_buffer_memory(self.vk_buffer(), vk_device_memory, offset)
    }
}
