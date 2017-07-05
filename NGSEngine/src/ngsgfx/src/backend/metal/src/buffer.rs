//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;

use {OCPtr, RefEqArc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Buffer {
    data: RefEqArc<BufferData>,
}

impl core::Buffer for Buffer {}

impl core::Marker for Buffer {
    fn set_label(&self, label: Option<&str>) {
        self.data.metal_buffer.set_label(label.unwrap_or(""));
    }
}

#[derive(Debug)]
struct BufferData {
    metal_buffer: OCPtr<metal::MTLBuffer>,
    size: u64,
}

unsafe impl Send for BufferData {}
unsafe impl Sync for BufferData {} // no interior mutability

impl Buffer {
    pub(crate) fn new(
        device: metal::MTLDevice,
        desc: &core::BufferDescription,
    ) -> core::Result<Self> {
        let options: metal::MTLResourceOptions = match desc.storage_mode {
            core::StorageMode::Private => metal::MTLResourceStorageModePrivate,
            core::StorageMode::Shared => metal::MTLResourceStorageModeShared,
            core::StorageMode::Memoryless => {
                // should have been filtered out by core::Validate
                unreachable!()
            }
        };
        let metal_buffer = unsafe { OCPtr::from_raw(device.new_buffer(desc.size, options)) }
            .ok_or(core::GenericError::OutOfDeviceMemory)?;
        let data = BufferData {
            metal_buffer: metal_buffer,
            size: desc.size,
        };
        Ok(Self { data: RefEqArc::new(data) })
    }

    pub(crate) unsafe fn contents(&self) -> *mut ::std::os::raw::c_void {
        ::std::mem::transmute(self.data.metal_buffer.contents())
    }

    pub(crate) fn len(&self) -> u64 {
        self.data.size
    }

    pub fn metal_buffer(&self) -> metal::MTLBuffer {
        *self.data.metal_buffer
    }
}
