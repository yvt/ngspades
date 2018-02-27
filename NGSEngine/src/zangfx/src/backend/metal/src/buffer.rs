//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Buffer` for Metal.
use base::{handles, resources, DeviceSize};
use common::{Error, ErrorKind, Result};
use metal;

use utils::OCPtr;

/// Implementation of `BufferBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct BufferBuilder {
    size: Option<DeviceSize>,
}

zangfx_impl_object! { BufferBuilder: resources::BufferBuilder, ::Debug }

impl BufferBuilder {
    /// Construct a `BufferBuilder`.
    pub fn new() -> Self {
        Self { size: None }
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
        Ok(handles::Buffer::new(Buffer::new(size)))
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
enum BufferData {
    Prototype(DeviceSize),
    Allocated(OCPtr<metal::MTLBuffer>),
}

impl Buffer {
    fn new(size: DeviceSize) -> Self {
        let data = BufferData::Prototype(size);
        Self {
            data: Box::into_raw(Box::new(data)),
        }
    }

    /// Construct a `Buffer` from a given raw `MTLBuffer`.
    ///
    /// The constructed `Buffer` will be initally in the Allocated state.
    pub unsafe fn from_raw(metal_buffer: metal::MTLBuffer) -> Self {
        let data = BufferData::Allocated(OCPtr::from_raw(metal_buffer).unwrap());
        Self {
            data: Box::into_raw(Box::new(data)),
        }
    }

    /// Return the underlying `MTLBuffer`. Returns `nil` for `Buffer`s in the
    /// Prototype state (i.e. not allocated on a heap).
    pub fn metal_buffer(&self) -> metal::MTLBuffer {
        match unsafe { &*self.data } {
            &BufferData::Prototype(_) => metal::MTLBuffer::nil(),
            &BufferData::Allocated(ref p) => **p,
        }
    }

    pub(super) fn prototype_size(&self) -> Option<DeviceSize> {
        match unsafe { &*self.data } {
            &BufferData::Prototype(size) => Some(size),
            &BufferData::Allocated(_) => None,
        }
    }

    pub(super) fn materialize(&self, metal_buffer: OCPtr<metal::MTLBuffer>) {
        unsafe {
            *self.data = BufferData::Allocated(metal_buffer);
        }
    }

    pub(super) unsafe fn destroy(&self) {
        Box::from_raw(self.data);
    }
}
