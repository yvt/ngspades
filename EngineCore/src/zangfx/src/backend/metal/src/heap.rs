//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Heap` for Metal.
use iterpool::{Pool, PoolPtr};
use parking_lot::Mutex;
use std::cell::UnsafeCell;
use std::sync::Arc;
use xalloc::{SysTlsf, SysTlsfRegion};
use zangfx_metal_rs as metal;

use zangfx_base::Result;
use zangfx_base::{self as base, heap, zangfx_impl_object, DeviceSize, MemoryType};

use crate::buffer::Buffer;
use crate::image::Image;
use crate::utils::{get_memory_req, nil_error, translate_storage_mode, OCPtr};

/// Implementation of `DynamicHeapBuilder` and `DedicatedHeapBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct HeapBuilder {
    metal_device: OCPtr<metal::MTLDevice>,
    size: DeviceSize,
    memory_type: Option<MemoryType>,
    label: Option<String>,
    bindings: Vec<Resource>,
}

#[derive(Debug, Clone)]
enum Resource {
    Image(base::ImageRef),
    Buffer(base::BufferRef),
}

impl Resource {
    fn clone_from(x: base::ResourceRef<'_>) -> Self {
        match x {
            base::ResourceRef::Image(x) => Resource::Image(x.clone()),
            base::ResourceRef::Buffer(x) => Resource::Buffer(x.clone()),
        }
    }

    fn as_ref(&self) -> base::ResourceRef<'_> {
        match self {
            Resource::Image(ref x) => base::ResourceRef::Image(x),
            Resource::Buffer(ref x) => base::ResourceRef::Buffer(x),
        }
    }
}

zangfx_impl_object! { HeapBuilder:
dyn heap::DynamicHeapBuilder, dyn heap::DedicatedHeapBuilder, dyn crate::Debug, dyn base::SetLabel }

unsafe impl Send for HeapBuilder {}
unsafe impl Sync for HeapBuilder {}

impl HeapBuilder {
    /// Construct a `HeapBuilder`.
    ///
    /// It's up to the caller to make sure `metal_device` is valid.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device: OCPtr::new(metal_device).expect("nil device"),
            size: 0,
            memory_type: None,
            label: None,
            bindings: Vec::new(),
        }
    }

    fn build_common(&mut self) -> Result<heap::HeapRef> {
        let memory_type = self.memory_type.expect("memory_type");
        let storage_mode = translate_storage_mode(memory_type).expect("memory_type");

        if self.size == 0 {
            panic!("size is zero");
        }

        let heap: heap::HeapRef;
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

            heap = Arc::new(Heap::new(metal_heap, storage_mode));
        } else {
            // `MTLHeap` only supports the private storage mode. So create a
            //  `MTLBuffer` and suballocate from it
            let options =
                metal::MTLResourceStorageModeShared | metal::MTLResourceHazardTrackingModeUntracked;
            let metal_buffer =
                unsafe { OCPtr::from_raw(self.metal_device.new_buffer(self.size, options)) }
                    .ok_or(nil_error("MTLDevice newBufferWithLength:options:"))?;
            heap = Arc::new(BufferHeap::new(metal_buffer));
        }

        // Dedicated allocations
        for resource in self.bindings.drain(..) {
            let success = heap.bind(resource.as_ref())?;
            assert!(success, "dedicated allocation failed for an unknown reason");
        }

        Ok(heap)
    }
}

impl base::SetLabel for HeapBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
    }
}

impl heap::DynamicHeapBuilder for HeapBuilder {
    fn size(&mut self, v: DeviceSize) -> &mut dyn heap::DynamicHeapBuilder {
        self.size = v;
        self
    }

    fn memory_type(&mut self, v: MemoryType) -> &mut dyn heap::DynamicHeapBuilder {
        self.memory_type = Some(v);
        self
    }

    fn build(&mut self) -> Result<heap::HeapRef> {
        self.build_common()
    }
}

impl heap::DedicatedHeapBuilder for HeapBuilder {
    fn queue(&mut self, _queue: &base::CmdQueueRef) -> &mut dyn base::DedicatedHeapBuilder {
        self
    }

    fn bind(&mut self, obj: base::ResourceRef<'_>) {
        let binding = Resource::clone_from(obj);
        self.bindings.reserve(1);

        let req = get_memory_req(obj).unwrap();
        self.size = (self.size + req.align - 1) & !(req.align - 1);
        self.size += req.size;

        // Add the resource handle to the dedicated allocation list
        self.bindings.push(binding);
    }

    fn enable_use_heap(&mut self) -> &mut dyn base::DedicatedHeapBuilder {
        self
    }

    fn memory_type(&mut self, v: MemoryType) -> &mut dyn heap::DedicatedHeapBuilder {
        self.memory_type = Some(v);
        self
    }

    fn build(&mut self) -> Result<heap::HeapRef> {
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

// zangfx_impl_handle! { HeapAlloc, base::HeapAlloc }

unsafe impl Send for HeapAlloc {}
unsafe impl Sync for HeapAlloc {}

/// Implementation of `Heap` for Metal, backed by `MTLHeap`.
#[derive(Debug)]
pub struct Heap {
    metal_heap: OCPtr<metal::MTLHeap>,
    storage_mode: metal::MTLStorageMode,
}

zangfx_impl_object! { Heap: dyn heap::Heap, dyn crate::Debug }

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
    fn bind(&self, obj: base::ResourceRef<'_>) -> Result<bool> {
        match obj {
            base::ResourceRef::Buffer(buffer) => {
                let metal_buffer_or_none =
                    bind_buffer(buffer, self.storage_mode, |size, options| {
                        self.metal_heap.new_buffer(size, options)
                    })?;

                Ok(metal_buffer_or_none.is_some())
            }

            base::ResourceRef::Image(image) => {
                let metal_texture_or_none = bind_image(image, self.storage_mode, |desc| {
                    self.metal_heap.new_texture(desc)
                })?;

                Ok(metal_texture_or_none.is_some())
            }
        }
    }

    fn make_aliasable(&self, obj: base::ResourceRef<'_>) -> Result<()> {
        match obj {
            base::ResourceRef::Buffer(buffer) => {
                let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
                my_buffer
                    .metal_buffer_and_offset()
                    .expect("not bound")
                    .0
                    .make_aliasable();
            }
            base::ResourceRef::Image(image) => {
                let my_image: &Image = image.downcast_ref().expect("bad image type");
                my_image.metal_texture().make_aliasable();
            }
        }
        Ok(())
    }
}

/// Implementation of `Heap` for Metal. It represents a global heap and
/// allocates resources from `MTLDevice` directly.
///
/// It does not support `use_heap`. Also, it does not support `make_aliasable`
/// as per the requirements of global heaps.
#[derive(Debug)]
pub struct GlobalHeap {
    metal_device: OCPtr<metal::MTLDevice>,
    storage_mode: metal::MTLStorageMode,
}

zangfx_impl_object! { GlobalHeap: dyn heap::Heap, dyn crate::Debug }

unsafe impl Send for GlobalHeap {}
unsafe impl Sync for GlobalHeap {}

impl GlobalHeap {
    pub unsafe fn new(metal_device: metal::MTLDevice, storage_mode: metal::MTLStorageMode) -> Self {
        Self {
            metal_device: OCPtr::new(metal_device).expect("nil device"),
            storage_mode,
        }
    }
}

impl heap::Heap for GlobalHeap {
    fn bind(&self, obj: base::ResourceRef<'_>) -> Result<bool> {
        match obj {
            base::ResourceRef::Buffer(buffer) => {
                let metal_buffer_or_none =
                    bind_buffer(buffer, self.storage_mode, |size, options| {
                        self.metal_device.new_buffer(size, options)
                    })?;

                Ok(metal_buffer_or_none.is_some())
            }

            base::ResourceRef::Image(image) => {
                let metal_image_or_none = bind_image(image, self.storage_mode, |desc| {
                    self.metal_device.new_texture(desc)
                })?;

                Ok(metal_image_or_none.is_some())
            }
        }
    }

    fn make_aliasable(&self, _resource: base::ResourceRef<'_>) -> Result<()> {
        // Global heaps do not support `make_aliasable`.
        Ok(())
    }
}

fn bind_buffer<T>(
    buffer: &base::BufferRef,
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
    )
    .unwrap()
        | metal::MTLResourceHazardTrackingModeUntracked;
    let metal_buffer = unsafe { OCPtr::from_raw(allocator(size, options)) };

    if let Some(metal_buffer) = metal_buffer {
        let metal_buffer_ptr = *metal_buffer;

        // Transition the buffer to the Allocated state
        my_buffer.materialize(metal_buffer, 0, None);

        // Return `metal_buffer_ptr` for `HeapAlloc` creation
        Ok(Some(metal_buffer_ptr))
    } else {
        Ok(None)
    }
}

fn bind_image<T>(
    image: &base::ImageRef,
    storage_mode: metal::MTLStorageMode,
    allocator: T,
) -> Result<Option<metal::MTLTexture>>
where
    T: FnOnce(metal::MTLTextureDescriptor) -> metal::MTLTexture,
{
    let my_image: &Image = image.downcast_ref().expect("bad image type");

    assert_eq!(storage_mode, metal::MTLStorageMode::Private);

    let metal_texture = unsafe { OCPtr::from_raw(allocator(my_image.prototype_metal_desc())) };

    if let Some(metal_texture) = metal_texture {
        let metal_texture_ptr = *metal_texture;

        // Transition the buffer to the Allocated state
        my_image.materialize(metal_texture);

        // Return `metal_texture_ptr` for `HeapAlloc` creation
        Ok(Some(metal_texture_ptr))
    } else {
        Ok(None)
    }
}

/// Implementation of `Heap` for Metal, backed by `MTLBuffer`.
#[derive(Debug)]
pub struct BufferHeap {
    metal_buffer: OCPtr<metal::MTLBuffer>,
    data: Mutex<BufferHeapData>,
}

zangfx_impl_object! { BufferHeap: dyn heap::Heap, dyn crate::Debug }

unsafe impl Send for BufferHeap {}
unsafe impl Sync for BufferHeap {}

#[derive(Debug)]
struct BufferHeapData {
    tlsf: SysTlsf<u32>,
    pool: Pool<Option<SysTlsfRegion>>,
}

/// Represents a single allocated region within a [`BufferHeap`].
#[derive(Debug)]
crate struct BufferHeapAlloc {
    /// Associates this `BufferHeapAlloc` with an element of
    /// `BufferHeapData::pool`.
    pool_ptr: UnsafeCell<Option<PoolPtr>>,
}

impl BufferHeap {
    fn new(metal_buffer: OCPtr<metal::MTLBuffer>) -> Self {
        let size = metal_buffer.length();

        // IINM Metal doesn't allow the creation of extremely large `MTLBuffer`s
        assert!(size <= 0x80000000);

        Self {
            metal_buffer,
            data: Mutex::new(BufferHeapData {
                tlsf: SysTlsf::new(size as u32),
                pool: Pool::new(),
            }),
        }
    }

    pub fn metal_buffer(&self) -> metal::MTLBuffer {
        *self.metal_buffer
    }
}

impl heap::Heap for BufferHeap {
    fn bind(&self, obj: base::ResourceRef<'_>) -> Result<bool> {
        use zangfx_base::Buffer as _Buffer; // for `get_memory_req`
        match obj {
            base::ResourceRef::Buffer(buffer) => {
                let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
                let memory_req = my_buffer.get_memory_req().unwrap();

                let mut data = self.data.lock();

                // Allocate the region
                if memory_req.size >= 0x8000_0000 {
                    // Does not fit in 32 bits
                    return Ok(false);
                }
                data.pool.reserve(1);
                let result = data
                    .tlsf
                    .alloc_aligned(memory_req.size as u32, memory_req.align as u32);

                if let Some((region, offset)) = result {
                    let pool_ptr = data.pool.allocate(Some(region));

                    let suballoc_info = BufferHeapAlloc {
                        pool_ptr: UnsafeCell::new(Some(pool_ptr)),
                    };

                    // Transition the buffer to the Allocated state
                    my_buffer.materialize(
                        self.metal_buffer.clone(),
                        offset as u64,
                        Some(suballoc_info),
                    );
                    Ok(true)
                } else {
                    Ok(false)
                }
            }

            base::ResourceRef::Image(_image) => {
                panic!("BufferHeap does not support binding image resources");
            }
        }
    }

    fn make_aliasable(&self, resource: base::ResourceRef<'_>) -> Result<()> {
        let my_alloc: &BufferHeapAlloc;

        match resource {
            base::ResourceRef::Buffer(buffer) => {
                let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
                my_alloc = my_buffer
                    .suballoc_info()
                    .expect("not allocated from a BufferHeap");
            }
            base::ResourceRef::Image(_) => panic!("not allocated from a BufferHeap"),
        }

        let mut data = self.data.lock();
        let ref mut data = *data; // Enable split borrows

        // Assuming the user obeys to the valid usage "`obj` must be bound to
        // this heap.", this should not cause a race condition since we are
        // already protected by a mutex
        let pool_ptr_cell = unsafe { &mut *my_alloc.pool_ptr.get() };

        // `make_aliasable` is idempotent
        if let Some(pool_ptr) = pool_ptr_cell.take() {
            let region = data.pool[pool_ptr].take().unwrap();
            unsafe {
                data.tlsf.dealloc_unchecked(region);
            }
        }

        Ok(())
    }
}
