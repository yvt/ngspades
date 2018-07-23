//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Buffer` for Metal.
use zangfx_base::{self as base, resources, DeviceSize};
use zangfx_base::Result;
use zangfx_base::{zangfx_impl_object, interfaces, vtable_for, zangfx_impl_handle};
use metal;

use utils::OCPtr;

/// Implementation of `BufferBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct BufferBuilder {
    size: Option<DeviceSize>,
    label: Option<String>,
    usage: base::BufferUsageFlags,
}

zangfx_impl_object! { BufferBuilder: resources::BufferBuilder, ::Debug, base::SetLabel }

impl BufferBuilder {
    /// Construct a `BufferBuilder`.
    pub fn new() -> Self {
        Self {
            size: None,
            label: None,
            usage: base::BufferUsage::default_flags(),
        }
    }
}

impl base::SetLabel for BufferBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
    }
}

impl resources::BufferBuilder for BufferBuilder {
    fn queue(&mut self, _queue: &base::CmdQueueRef) -> &mut base::BufferBuilder {
        self
    }

    fn size(&mut self, v: DeviceSize) -> &mut resources::BufferBuilder {
        self.size = Some(v);
        self
    }
    fn usage(&mut self, v: resources::BufferUsageFlags) -> &mut resources::BufferBuilder {
        self.usage = v;
        self
    }

    fn build(&mut self) -> Result<base::BufferRef> {
        let size = self.size.expect("size");
        Ok(Buffer::new(
            size,
            self.label.clone(),
            self.usage,
        ).into())
    }
}

/// Implementation of `Buffer` for Metal.
#[derive(Debug, Clone)]
pub struct Buffer {
    data: *mut BufferData,
}

zangfx_impl_handle! { Buffer, base::BufferRef }

unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

#[derive(Debug)]
struct BufferData {
    size: DeviceSize,
    metal_buffer: Option<(OCPtr<metal::MTLBuffer>, base::DeviceSize, bool)>,
    usage: base::BufferUsageFlags,
    label: Option<String>,
}

impl Buffer {
    fn new(size: DeviceSize, label: Option<String>, usage: base::BufferUsageFlags) -> Self {
        let data = BufferData {
            size,
            metal_buffer: None,
            usage,
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
            metal_buffer: Some((OCPtr::from_raw(metal_buffer).unwrap(), 0, false)),
            usage: base::BufferUsageFlags::all(),
            label: None,
        };

        Self {
            data: Box::into_raw(Box::new(data)),
        }
    }

    unsafe fn data(&self) -> &mut BufferData {
        &mut *self.data
    }

    /// Return the underlying `MTLBuffer`. Returns `None` for `Buffer`s in the
    /// Prototype state (i.e. not allocated on a heap).
    pub fn metal_buffer_and_offset(&self) -> Option<(metal::MTLBuffer, base::DeviceSize)> {
        unsafe {
            self.data()
                .metal_buffer
                .as_ref()
                .map(|&(ref p, offset, _)| (**p, offset))
        }
    }

    pub(super) fn size(&self) -> DeviceSize {
        unsafe { self.data().size }
    }

    pub(super) fn is_subbuffer(&self) -> bool {
        unsafe { self.data().metal_buffer.as_ref().unwrap().2 }
    }

    /// Assign a `MTLBuffer` to this `Buffer` object.
    ///
    /// `is_subbuffer` indicates whether `metal_buffer` is a subportion of a
    /// larger `MTLBuffer` used to realize a heap.
    pub(super) fn materialize(
        &self,
        metal_buffer: OCPtr<metal::MTLBuffer>,
        offset: base::DeviceSize,
        is_subbuffer: bool,
    ) {
        let data = unsafe { self.data() };
        data.metal_buffer = Some((metal_buffer, offset, is_subbuffer));

        if let Some(label) = data.label.take() {
            data.metal_buffer.as_ref().unwrap().0.set_label(&label);
        }
    }

    pub(super) fn memory_req(&self, metal_device: metal::MTLDevice) -> resources::MemoryReq {
        let mut metal_req = metal_device.heap_buffer_size_and_align_with_length(
            self.size(),
            metal::MTLResourceStorageModePrivate | metal::MTLResourceHazardTrackingModeUntracked,
        );

        use std::cmp::max;
        let usage = unsafe { self.data() }.usage;
        if usage.contains(base::BufferUsage::Storage) {
            metal_req.align = max(metal_req.align, ::STORAGE_BUFFER_MIN_ALIGN);
        }
        if usage.contains(base::BufferUsage::Uniform) {
            metal_req.align = max(metal_req.align, ::UNIFORM_BUFFER_MIN_ALIGN);
        }

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

impl base::Buffer for Buffer {
    fn as_ptr(&self) -> *mut u8 {
        unimplemented!()
    }

    fn get_memory_req(&self) -> Result<base::MemoryReq> {
        unimplemented!()
    }
}