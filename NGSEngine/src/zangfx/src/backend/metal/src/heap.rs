//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Heap` for Metal.
use metal;
use iterpool::{IterablePool, PoolPtr};
use parking_lot::Mutex;

use base::{self, handles, heap, DeviceSize, MemoryType};
use common::{Error, ErrorKind, Result};

use utils::{get_memory_req, nil_error, translate_storage_mode, OCPtr};
use buffer::Buffer;

/// Implementation of `DynamicHeapBuilder` and `DedicatedHeapBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct HeapBuilder {
    /// A reference to a `MTLDevice`. We are not required to maintain a strong
    /// reference. (See the base interface's documentation)
    metal_device: metal::MTLDevice,
    size: DeviceSize,
    memory_type: Option<MemoryType>,
    label: Option<String>,
}

zangfx_impl_object! { HeapBuilder:
heap::DynamicHeapBuilder, heap::DedicatedHeapBuilder, ::Debug }

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
            label: None,
        }
    }

    fn build_common(&mut self) -> Result<Box<heap::Heap>> {
        let memory_type = self.memory_type
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "memory_type"))?;
        let storage_mode = translate_storage_mode(memory_type)
            .map_err(|_| Error::with_detail(ErrorKind::InvalidUsage, "memory_type"))?;

        if self.size == 0 {
            return Err(Error::new(ErrorKind::InvalidUsage));
        }

        if storage_mode == metal::MTLStorageMode::Private {
            let metal_desc = unsafe { OCPtr::from_raw(metal::MTLHeapDescriptor::new()) }
                .ok_or(nil_error("MTLHeapDescriptor new"))?;
            metal_desc.set_size(self.size);
            metal_desc.set_storage_mode(storage_mode);

            let metal_heap = OCPtr::new(self.metal_device.new_heap(*metal_desc))
                .ok_or_else(|| nil_error("MTLDevice newHeapWithDescriptor:"))?;

            if let Some(ref label) = self.label {
                metal_heap.set_label(label);
            }

            Ok(Box::new(Heap::new(metal_heap, storage_mode)))
        } else {
            // `MTLHeap` only supports the private storage mode
            Ok(Box::new(unsafe {
                EmulatedHeap::new(self.metal_device, storage_mode)
            }))
        }
    }
}

impl base::SetLabel for HeapBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
    }
}

impl heap::DynamicHeapBuilder for HeapBuilder {
    fn size(&mut self, v: DeviceSize) -> &mut heap::DynamicHeapBuilder {
        self.size = v;
        self
    }

    fn memory_type(&mut self, v: MemoryType) -> &mut heap::DynamicHeapBuilder {
        self.memory_type = Some(v);
        self
    }

    fn build(&mut self) -> Result<Box<heap::Heap>> {
        self.build_common()
    }
}

impl heap::DedicatedHeapBuilder for HeapBuilder {
    fn prebind(&mut self, obj: handles::ResourceRef) {
        let req = get_memory_req(self.metal_device, obj).unwrap();
        self.size = (self.size + req.align - 1) & !(req.align - 1);
        self.size += req.size;
    }

    fn memory_type(&mut self, v: MemoryType) -> &mut heap::DedicatedHeapBuilder {
        self.memory_type = Some(v);
        self
    }

    fn build(&mut self) -> Result<Box<heap::Heap>> {
        self.build_common()
    }
}

/// Implementation of `HeapAlloc` for Metal. To be used with [`Heap`].
///
/// [`Heap`]: Heap
#[derive(Debug, Clone)]
pub struct HeapAlloc {
    resource: metal::MTLResource,
}

zangfx_impl_handle! { HeapAlloc, handles::HeapAlloc }

unsafe impl Send for HeapAlloc {}
unsafe impl Sync for HeapAlloc {}

/// Implementation of `Heap` for Metal, backed by `MTLHeap`.
#[derive(Debug)]
pub struct Heap {
    metal_heap: OCPtr<metal::MTLHeap>,
    storage_mode: metal::MTLStorageMode,
}

zangfx_impl_object! { Heap: heap::Heap, ::Debug }

unsafe impl Send for Heap {}
unsafe impl Sync for Heap {}

impl Heap {
    fn new(metal_heap: OCPtr<metal::MTLHeap>, storage_mode: metal::MTLStorageMode) -> Self {
        Self {
            metal_heap,
            storage_mode,
        }
    }

    pub fn metal_heap(&self) -> metal::MTLHeap {
        *self.metal_heap
    }
}

impl heap::Heap for Heap {
    fn bind(&self, obj: handles::ResourceRef) -> Result<Option<handles::HeapAlloc>> {
        match obj {
            handles::ResourceRef::Buffer(buffer) => {
                let metal_buffer_or_none =
                    bind_buffer(buffer, self.storage_mode, |size, options| {
                        self.metal_heap.new_buffer(size, options)
                    })?;

                Ok(metal_buffer_or_none.map(|metal_buffer| {
                    // If the allocation was successful, then return
                    // a `HeapAlloc` for the allocated buffer
                    let resource = *metal_buffer;
                    let heap_alloc = HeapAlloc { resource };

                    handles::HeapAlloc::new(heap_alloc)
                }))
            }

            handles::ResourceRef::Image(_image) => unimplemented!(),
        }
    }

    fn make_aliasable(&self, alloc: &handles::HeapAlloc) -> Result<()> {
        let my_alloc: &HeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        my_alloc.resource.make_aliasable();
        Ok(())
    }

    fn unbind(&self, alloc: &handles::HeapAlloc) -> Result<()> {
        let my_alloc: &HeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");

        // Deallocate the resource as soon as possible
        my_alloc.resource.make_aliasable();

        Ok(())
    }

    fn as_ptr(&self, _alloc: &handles::HeapAlloc) -> Result<*mut ()> {
        Err(Error::with_detail(
            ErrorKind::InvalidUsage,
            "not host visible",
        ))
    }
}

/// Implementation of `EmulatedHeapAlloc` for Metal. To be used with [`EmulatedHeap`].
///
/// [`EmulatedHeap`]: EmulatedHeap
#[derive(Debug, Clone)]
pub struct EmulatedHeapAlloc {
    /// The pointer to the resource's contents. Invalid for images.
    contents_ptr: *mut (),

    /// Associates this `EmulatedHeapAlloc` with an element of
    /// `EmulatedHeap::pool`.
    pool_ptr: PoolPtr,
}

zangfx_impl_handle! { EmulatedHeapAlloc, handles::HeapAlloc }

unsafe impl Send for EmulatedHeapAlloc {}
unsafe impl Sync for EmulatedHeapAlloc {}

/// Emulated implementation of `Heap` for Metal. Does not `MTLHeap` and
/// allocates resources from `MTLDevice` directly.
#[derive(Debug)]
pub struct EmulatedHeap {
    metal_device: metal::MTLDevice,
    storage_mode: metal::MTLStorageMode,

    /// We need to keep the list of allocated resources to implement
    /// `CmdEncoder::use_heap`.
    pool: Mutex<IterablePool<metal::MTLResource>>,
}

zangfx_impl_object! { EmulatedHeap: heap::Heap, ::Debug }

unsafe impl Send for EmulatedHeap {}
unsafe impl Sync for EmulatedHeap {}

impl EmulatedHeap {
    unsafe fn new(metal_device: metal::MTLDevice, storage_mode: metal::MTLStorageMode) -> Self {
        Self {
            metal_device,
            storage_mode,
            pool: Mutex::new(IterablePool::new()),
        }
    }

    pub(crate) fn for_each_metal_resources<T>(&self, cb: &mut T)
    where
        T: FnMut(metal::MTLResource),
    {
        for &metal_resource in self.pool.lock().iter() {
            cb(metal_resource);
        }
    }
}

impl heap::Heap for EmulatedHeap {
    fn bind(&self, obj: handles::ResourceRef) -> Result<Option<handles::HeapAlloc>> {
        match obj {
            handles::ResourceRef::Buffer(buffer) => {
                let metal_buffer_or_none =
                    bind_buffer(buffer, self.storage_mode, |size, options| {
                        self.metal_device.new_buffer(size, options)
                    })?;

                Ok(metal_buffer_or_none.map(|metal_buffer| {
                    // If the allocation was successful, then return
                    // a `HeapAlloc` for the allocated buffer
                    let contents_ptr = metal_buffer.contents() as *mut ();
                    let pool_ptr = self.pool.lock().allocate(*metal_buffer);

                    let heap_alloc = EmulatedHeapAlloc {
                        contents_ptr,
                        pool_ptr,
                    };

                    handles::HeapAlloc::new(heap_alloc)
                }))
            }

            handles::ResourceRef::Image(_image) => unimplemented!(),
        }
    }

    fn make_aliasable(&self, _alloc: &handles::HeapAlloc) -> Result<()> {
        // We do not support aliasing, but the definition of `make_aliasable`
        // does not guarantee aliasing
        Ok(())
    }

    fn unbind(&self, alloc: &handles::HeapAlloc) -> Result<()> {
        let my_alloc: &EmulatedHeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        self.pool.lock().deallocate(my_alloc.pool_ptr).unwrap();

        // We do not maintain the lifetime of `MTLResource`
        Ok(())
    }

    fn as_ptr(&self, alloc: &handles::HeapAlloc) -> Result<*mut ()> {
        let my_alloc: &EmulatedHeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        Ok(my_alloc.contents_ptr)
    }
}

fn bind_buffer<T>(
    buffer: &handles::Buffer,
    storage_mode: metal::MTLStorageMode,
    allocator: T,
) -> Result<Option<metal::MTLBuffer>>
where
    T: FnOnce(u64, metal::MTLResourceOptions) -> metal::MTLBuffer,
{
    let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");

    let size = my_buffer.size();

    let options = metal::MTLResourceOptions::from_bits(
        (storage_mode as u64) << metal::MTLResourceStorageModeShift,
    ).unwrap() | metal::MTLResourceHazardTrackingModeUntracked;
    let metal_buffer = OCPtr::new(allocator(size, options));

    if let Some(metal_buffer) = metal_buffer {
        let metal_buffer_ptr = *metal_buffer;

        // Transition the buffer to the Allocated state
        my_buffer.materialize(metal_buffer);

        // Return `metal_buffer_ptr` for `HeapAlloc` creation
        Ok(Some(metal_buffer_ptr))
    } else {
        Ok(None)
    }
}
