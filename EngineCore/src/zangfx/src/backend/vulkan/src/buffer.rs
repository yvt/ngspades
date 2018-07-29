//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Buffer` for Vulkan.
use ash::version::*;
use ash::vk;
use std::sync::Arc;

use crate::device::DeviceRef;
use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_handle, zangfx_impl_object};

use crate::resstate;
use crate::utils::{queue_id_from_queue, translate_generic_error_unwrap, QueueIdBuilder};

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
            usage: base::BufferUsage::default_flags(),
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
        if self.usage.contains(base::BufferUsage::Vertex) {
            usage |= vk::BUFFER_USAGE_VERTEX_BUFFER_BIT;
        }
        if self.usage.contains(base::BufferUsage::CopyRead) {
            usage |= vk::BUFFER_USAGE_TRANSFER_SRC_BIT;
        }
        if self.usage.contains(base::BufferUsage::CopyWrite) {
            usage |= vk::BUFFER_USAGE_TRANSFER_DST_BIT;
        }
        if self.usage.contains(base::BufferUsage::Uniform) {
            usage |= vk::BUFFER_USAGE_UNIFORM_BUFFER_BIT;
        }
        if self.usage.contains(base::BufferUsage::Storage) {
            usage |= vk::BUFFER_USAGE_STORAGE_BUFFER_BIT;
        }
        if self.usage.contains(base::BufferUsage::Index) {
            usage |= vk::BUFFER_USAGE_INDEX_BUFFER_BIT;
        }
        if self.usage.contains(base::BufferUsage::IndirectDraw) {
            usage |= vk::BUFFER_USAGE_INDIRECT_BUFFER_BIT;
        }

        let info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BufferCreateInfo,
            p_next: ::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: size,
            usage,
            sharing_mode: vk::SharingMode::Exclusive,
            queue_family_index_count: 0, // ignored for `SharingMode::Exclusive`
            p_queue_family_indices: ::null(),
        };

        let device = self.device.clone();
        let vk_buffer = unsafe {
            let vk_device = device.vk_device();
            vk_device.create_buffer(&info, None)
        }.map_err(translate_generic_error_unwrap)?;

        let vulkan_buffer = Arc::new(VulkanBuffer {
            device,
            vk_buffer,
            len: size,
        });

        let queue_id = self.queue_id.get(&vulkan_buffer.device);
        let tracked_state = Arc::new(resstate::TrackedState::new(queue_id, ()));

        Ok(Buffer {
            vulkan_buffer,
            tracked_state,
        }.into())
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
    // TODO: Heap binding
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

unsafe impl base::Buffer for Buffer {
    fn as_ptr(&self) -> *mut u8 {
        unimplemented!()
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
        }.into()
    }

    fn get_memory_req(&self) -> Result<base::MemoryReq> {
        unimplemented!()
    }
}
