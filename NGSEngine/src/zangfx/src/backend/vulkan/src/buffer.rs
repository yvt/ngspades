//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Buffer` for Vulkan.
use ash::vk;
use ash::version::*;

use base;
use common::{Error, ErrorKind, Result};
use device::DeviceRef;

use utils::translate_generic_error_unwrap;

/// Implementation of `BufferBuilder` for Vulkan.
#[derive(Debug)]
pub struct BufferBuilder {
    device: DeviceRef,
    size: Option<base::DeviceSize>,
    usage: base::BufferUsageFlags,
}

zangfx_impl_object! { BufferBuilder: base::BufferBuilder, ::Debug }

impl BufferBuilder {
    pub(super) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            size: None,
            usage: base::BufferUsage::default_flags(),
        }
    }
}

impl base::BufferBuilder for BufferBuilder {
    fn size(&mut self, v: base::DeviceSize) -> &mut base::BufferBuilder {
        self.size = Some(v);
        self
    }

    fn usage(&mut self, v: base::BufferUsageFlags) -> &mut base::BufferBuilder {
        self.usage = v;
        self
    }

    fn build(&mut self) -> Result<base::Buffer> {
        let size = self.size
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "size"))?;

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

        let vk_device = self.device.vk_device();
        let vk_buffer = unsafe { vk_device.create_buffer(&info, None) }
            .map_err(translate_generic_error_unwrap)?;
        Ok(Buffer { vk_buffer }.into())
    }
}

/// Implementation of `Buffer` for Vulkan.
#[derive(Debug, Clone)]
pub struct Buffer {
    vk_buffer: vk::Buffer,
}

zangfx_impl_handle! { Buffer, base::Buffer }

unsafe impl Sync for Buffer {}
unsafe impl Send for Buffer {}

impl Buffer {
    pub unsafe fn from_raw(vk_buffer: vk::Buffer) -> Self {
        Self { vk_buffer }
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        self.vk_buffer
    }

    pub(super) unsafe fn destroy(&self, vk_device: &::AshDevice) {
        vk_device.destroy_buffer(self.vk_buffer, None);
    }
}
