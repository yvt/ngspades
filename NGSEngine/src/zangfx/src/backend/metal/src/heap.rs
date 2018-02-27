//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Heap` for Metal.
use metal;
use iterpool::{Pool, PoolPtr};
use parking_lot::Mutex;

use base::{handles, heap, DeviceSize, MemoryType};
use common::{Error, ErrorKind, Result};

use utils::{nil_error, translate_storage_mode, OCPtr};

/// Implementation of `HeapBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct HeapBuilder {
    /// A reference to a `MTLDevice`. We are not required to maintain a strong
    /// reference. (See the base interface's documentation)
    metal_device: metal::MTLDevice,
    size: DeviceSize,
    memory_type: Option<MemoryType>,
}

zangfx_impl_object! { HeapBuilder: heap::HeapBuilder, ::Debug }

unsafe impl Send for HeapBuilder {}
unsafe impl Sync for HeapBuilder {}

impl HeapBuilder {
    /// Construct a `HeapBuilder`.
    ///
    /// Ir's up to the caller to maintain the lifetime of `metal_device`.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device,
            size: 0,
            memory_type: None,
        }
    }
}

impl heap::HeapBuilder for HeapBuilder {
    fn size(&mut self, v: DeviceSize) -> &mut heap::HeapBuilder {
        self.size = v;
        self
    }

    fn memory_type(&mut self, v: MemoryType) -> &mut heap::HeapBuilder {
        self.memory_type = Some(v);
        self
    }

    fn build(&mut self) -> Result<Box<heap::Heap>> {
        let memory_type = self.memory_type
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "memory_type"))?;
        let storage_mode = translate_storage_mode(memory_type)
            .map_err(|_| Error::with_detail(ErrorKind::InvalidUsage, "memory_type"))?;

        if self.size == 0 {
            return Err(Error::new(ErrorKind::InvalidUsage));
        }

        let metal_desc = unsafe { OCPtr::from_raw(metal::MTLHeapDescriptor::new()) }
            .ok_or(nil_error("MTLHeapDescriptor new"))?;
        metal_desc.set_size(self.size);
        metal_desc.set_storage_mode(storage_mode);

        let metal_heap = OCPtr::new(self.metal_device.new_heap(*metal_desc))
            .ok_or_else(|| nil_error("MTLDevice newHeapWithDescriptor:"))?;
        Ok(Box::new(Heap::new(metal_heap)))
    }
}

/// Implementation of `HeapAlloc` for Metal.
#[derive(Debug, Clone)]
pub struct HeapAlloc {
    ptr: PoolPtr,
}

zangfx_impl_handle! { HeapAlloc, handles::HeapAlloc }

/// Implementation of `Heap` for Metal.
#[derive(Debug)]
pub struct Heap {
    metal_heap: OCPtr<metal::MTLHeap>,
    allocations: Mutex<Pool<OCPtr<metal::MTLResource>>>,
}

zangfx_impl_object! { Heap: heap::Heap, ::Debug }

unsafe impl Send for Heap {}
unsafe impl Sync for Heap {}

impl Heap {
    fn new(metal_heap: OCPtr<metal::MTLHeap>) -> Self {
        Self {
            metal_heap,
            allocations: Mutex::new(Pool::new()),
        }
    }
}

impl heap::Heap for Heap {
    fn bind(&self, _obj: handles::ResourceRef) -> Result<Option<handles::HeapAlloc>> {
        unimplemented!()
    }

    fn make_aliasable(&self, alloc: &handles::HeapAlloc) -> Result<()> {
        let my_alloc: &HeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        self.allocations.lock()[my_alloc.ptr].make_aliasable();
        Ok(())
    }

    fn unbind(&self, alloc: &handles::HeapAlloc) -> Result<()> {
        let my_alloc: &HeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");

        // Deallocate the resource as soon as possible
        let mut allocations = self.allocations.lock();
        allocations[my_alloc.ptr].make_aliasable();

        allocations.deallocate(my_alloc.ptr);

        Ok(())
    }

    fn as_ptr(&self, alloc: &handles::HeapAlloc) -> Result<*mut ()> {
        use std::mem::transmute;
        let my_alloc: &HeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");

        let resource: metal::MTLResource = *self.allocations.lock()[my_alloc.ptr];

        // The associated resource must be a buffer
        let buffer: metal::MTLBuffer = unsafe { transmute(resource) };
        Ok(buffer.contents() as *mut ())
    }
}
