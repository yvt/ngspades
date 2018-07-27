//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Buffer` for Metal.
use std::cell::UnsafeCell;
use std::sync::Arc;

use zangfx_base::Result;
use zangfx_base::{self as base, resources, DeviceSize};
use zangfx_base::{interfaces, vtable_for, zangfx_impl_handle, zangfx_impl_object};
use zangfx_metal_rs as metal;

use crate::heap::BufferHeapAlloc;
use crate::utils::OCPtr;

/// Implementation of `BufferBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct BufferBuilder {
    metal_device: OCPtr<metal::MTLDevice>,
    size: Option<DeviceSize>,
    label: Option<String>,
    usage: base::BufferUsageFlags,
}

zangfx_impl_object! { BufferBuilder: dyn resources::BufferBuilder, dyn crate::Debug, dyn base::SetLabel }

unsafe impl Send for BufferBuilder {}
unsafe impl Sync for BufferBuilder {}

impl BufferBuilder {
    /// Construct a `BufferBuilder`.
    ///
    /// It's up to the caller to make sure `metal_device` is valid.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device: OCPtr::new(metal_device).expect("nil device"),
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
    fn queue(&mut self, _queue: &base::CmdQueueRef) -> &mut dyn base::BufferBuilder {
        self
    }

    fn size(&mut self, v: DeviceSize) -> &mut dyn resources::BufferBuilder {
        self.size = Some(v);
        self
    }
    fn usage(&mut self, v: resources::BufferUsageFlags) -> &mut dyn resources::BufferBuilder {
        self.usage = v;
        self
    }

    fn build(&mut self) -> Result<base::BufferRef> {
        let size = self.size.expect("size");
        Ok(Buffer::new(*self.metal_device, size, self.label.clone(), self.usage).into())
    }
}

/// Implementation of `Buffer` for Metal.
#[derive(Debug, Clone)]
pub struct Buffer {
    data: Arc<UnsafeCell<BufferData>>,
}

zangfx_impl_handle! { Buffer, base::BufferRef }

unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

#[derive(Debug)]
struct BufferData {
    size: DeviceSize,
    metal_buffer: Option<(
        OCPtr<metal::MTLBuffer>,
        base::DeviceSize,
        Option<BufferHeapAlloc>,
    )>,
    usage: base::BufferUsageFlags,
    memory_req: Option<base::MemoryReq>,
    label: Option<String>,
}

impl Buffer {
    fn new(
        metal_device: metal::MTLDevice,
        size: DeviceSize,
        label: Option<String>,
        usage: base::BufferUsageFlags,
    ) -> Self {
        // Compute the memory requirement
        let mut metal_req = metal_device.heap_buffer_size_and_align_with_length(
            size,
            metal::MTLResourceStorageModePrivate | metal::MTLResourceHazardTrackingModeUntracked,
        );

        use std::cmp::max;
        if usage.contains(base::BufferUsage::Storage) {
            metal_req.align = max(metal_req.align, crate::STORAGE_BUFFER_MIN_ALIGN);
        }
        if usage.contains(base::BufferUsage::Uniform) {
            metal_req.align = max(metal_req.align, crate::UNIFORM_BUFFER_MIN_ALIGN);
        }

        let memory_req = resources::MemoryReq {
            size: metal_req.size,
            align: metal_req.align,
            memory_types: crate::MEMORY_TYPE_ALL_BITS,
        };

        // Construct a handle
        let data = BufferData {
            size,
            metal_buffer: None,
            usage,
            memory_req: Some(memory_req),
            label,
        };

        Self {
            data: Arc::new(UnsafeCell::new(data)),
        }
    }

    /// Construct a `Buffer` from a given raw `MTLBuffer`.
    ///
    /// - The constructed `Buffer` will be initally in the Allocated state.
    /// - The constructed `Buffer` does not support `Buffer::get_memory_req`.
    pub unsafe fn from_raw(metal_buffer: metal::MTLBuffer) -> Self {
        let data = BufferData {
            size: metal_buffer.length(),
            metal_buffer: Some((OCPtr::from_raw(metal_buffer).unwrap(), 0, None)),
            usage: base::BufferUsageFlags::all(),
            memory_req: None,
            label: None,
        };

        Self {
            data: Arc::new(UnsafeCell::new(data)),
        }
    }

    unsafe fn data(&self) -> &mut BufferData {
        &mut *self.data.get()
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

    /// Return the suballocation info if this `Buffer` represents a region
    /// suballocated from a `BufferHeap`.
    pub(super) fn suballoc_info(&self) -> Option<&BufferHeapAlloc> {
        unsafe {
            self.data()
                .metal_buffer
                .as_ref()
                .expect("not bound")
                .2
                .as_ref()
        }
    }

    /// Determine whether this `Buffer` represents a region suballocated from a
    /// `BufferHeap` or not.
    pub(super) fn is_subbuffer(&self) -> bool {
        self.suballoc_info().is_some()
    }

    /// Assign a `MTLBuffer` to this `Buffer` object.
    ///
    /// If the pointed region was suballocated from of a larger `MTLBuffer`,
    /// `suballoc_info` specifies the suballocation info.
    pub(super) fn materialize(
        &self,
        metal_buffer: OCPtr<metal::MTLBuffer>,
        offset: base::DeviceSize,
        suballoc_info: Option<BufferHeapAlloc>,
    ) {
        let data = unsafe { self.data() };
        assert!(data.metal_buffer.is_none(), "already materialized");
        data.metal_buffer = Some((metal_buffer, offset, suballoc_info));

        if let Some(label) = data.label.take() {
            data.metal_buffer.as_ref().unwrap().0.set_label(&label);
        }
    }
}

unsafe impl base::Buffer for Buffer {
    fn as_ptr(&self) -> *mut u8 {
        let data = unsafe { self.data() };
        let (ref metal_buffer, ref offset, _) = data.metal_buffer.as_ref().expect("not bound");
        let contents_ptr = metal_buffer.contents() as *mut u8;
        if contents_ptr.is_null() {
            panic!("null pointer");
        }
        contents_ptr.wrapping_offset(*offset as isize)
    }

    fn get_memory_req(&self) -> Result<base::MemoryReq> {
        Ok(unsafe { self.data() }
            .memory_req
            .expect("This buffer does not support get_memory_req"))
    }

    fn len(&self) -> DeviceSize {
        let data = unsafe { self.data() };
        data.size
    }
}
