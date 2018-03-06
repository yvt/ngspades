//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Buffer` for Metal.
use base::{self, handles, resources, DeviceSize};
use common::{Error, ErrorKind, Result};
use metal;

use utils::OCPtr;

/// Implementation of `BufferBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct BufferBuilder {
    size: Option<DeviceSize>,
    label: Option<String>,
}

zangfx_impl_object! { BufferBuilder: resources::BufferBuilder, ::Debug }

impl BufferBuilder {
    /// Construct a `BufferBuilder`.
    pub fn new() -> Self {
        Self {
            size: None,
            label: None,
        }
    }
}

impl base::SetLabel for BufferBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
    }
}

impl resources::BufferBuilder for BufferBuilder {
    fn size(&mut self, v: DeviceSize) -> &mut resources::BufferBuilder {
        self.size = Some(v);
        self
    }
    fn usage(&mut self, _v: resources::BufferUsageFlags) -> &mut resources::BufferBuilder {
        self
    }

    fn build(&mut self) -> Result<handles::Buffer> {
        let size = self.size
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "size"))?;
        Ok(handles::Buffer::new(Buffer::new(size, self.label.clone())))
    }
}

/// Implementation of `Buffer` for Metal.
#[derive(Debug, Clone)]
pub struct Buffer {
    data: *mut BufferData,
}

zangfx_impl_handle! { Buffer, handles::Buffer }

unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

#[derive(Debug)]
struct BufferData {
    size: DeviceSize,
    metal_buffer: Option<OCPtr<metal::MTLBuffer>>,
    label: Option<String>,
}

impl Buffer {
    fn new(size: DeviceSize, label: Option<String>) -> Self {
        let data = BufferData {
            size,
            metal_buffer: None,
            label,
        };

        Self {
            data: Box::into_raw(Box::new(data)),
        }
    }

    /// Construct a `Buffer` from a given raw `MTLBuffer`.
    ///
    /// The constructed `Buffer` will be initally in the Allocated state.
    pub unsafe fn from_raw(metal_buffer: metal::MTLBuffer) -> Self {
        let data = BufferData {
            size: metal_buffer.length(),
            metal_buffer: OCPtr::from_raw(metal_buffer),
            label: None,
        };

        Self {
            data: Box::into_raw(Box::new(data)),
        }
    }

    unsafe fn data(&self) -> &mut BufferData {
        &mut *self.data
    }

    /// Return the underlying `MTLBuffer`. Returns `nil` for `Buffer`s in the
    /// Prototype state (i.e. not allocated on a heap).
    pub fn metal_buffer(&self) -> metal::MTLBuffer {
        unsafe {
            if let Some(ref p) = self.data().metal_buffer {
                **p
            } else {
                metal::MTLBuffer::nil()
            }
        }
    }

    pub(super) fn size(&self) -> DeviceSize {
        unsafe { self.data().size }
    }

    pub(super) fn materialize(&self, metal_buffer: OCPtr<metal::MTLBuffer>) {
        let data = unsafe { self.data() };
        data.metal_buffer = Some(metal_buffer);

        if let Some(label) = data.label.take() {
            data.metal_buffer.as_ref().unwrap().set_label(&label);
        }
    }

    pub(super) fn memory_req(&self, metal_device: metal::MTLDevice) -> resources::MemoryReq {
        let metal_req = metal_device.heap_buffer_size_and_align_with_length(
            self.size(),
            metal::MTLResourceStorageModePrivate | metal::MTLResourceHazardTrackingModeUntracked,
        );
        resources::MemoryReq {
            size: metal_req.size,
            align: metal_req.align,
            memory_types: ::MEMORY_TYPE_ALL_BITS,
        }
    }

    pub(super) unsafe fn destroy(&self) {
        Box::from_raw(self.data);
    }
}
