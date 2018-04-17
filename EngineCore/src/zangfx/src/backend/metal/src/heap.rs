//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Heap` for Metal.
use metal;
use iterpool::{IterablePool, Pool, PoolPtr};
use parking_lot::Mutex;
use xalloc::{SysTlsf, SysTlsfRegion};

use base::{self, handles, heap, DeviceSize, MemoryType};
use common::{Error, ErrorKind, Result};

use utils::{get_memory_req, nil_error, translate_storage_mode, OCPtr};
use buffer::Buffer;
use image::Image;

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
heap::DynamicHeapBuilder, heap::DedicatedHeapBuilder, ::Debug, base::SetLabel }

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
            // `MTLHeap` only supports the private storage mode. So create a
            //  `MTLBuffer` and suballocate from it
            let options =
                metal::MTLResourceStorageModeShared | metal::MTLResourceHazardTrackingModeUntracked;
            let metal_buffer = unsafe {
                OCPtr::from_raw(self.metal_device.new_buffer(self.size, options))
            }.ok_or(nil_error("MTLDevice newBufferWithLength:options:"))?;
            Ok(Box::new(BufferHeap::new(metal_buffer)))
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

            handles::ResourceRef::Image(image) => {
                let metal_texture_or_none = bind_image(image, self.storage_mode, |desc| {
                    self.metal_heap.new_texture(desc)
                })?;

                Ok(metal_texture_or_none.map(|metal_texture| {
                    // If the allocation was successful, then return
                    // a `HeapAlloc` for the allocated image
                    let resource = *metal_texture;
                    let heap_alloc = HeapAlloc { resource };

                    handles::HeapAlloc::new(heap_alloc)
                }))
            }
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

    fn as_ptr(&self, _alloc: &handles::HeapAlloc) -> Result<*mut u8> {
        Err(Error::with_detail(
            ErrorKind::InvalidUsage,
            "not host visible",
        ))
    }
}

/// Implementation of `HeapAlloc` for Metal. To be used with [`EmulatedHeap`].
///
/// [`EmulatedHeap`]: EmulatedHeap
#[derive(Debug, Clone)]
pub struct EmulatedHeapAlloc {
    /// The pointer to the resource's contents. Invalid for images.
    contents_ptr: *mut u8,

    /// Associates this `EmulatedHeapAlloc` with an element of
    /// `EmulatedHeap::pool`.
    pool_ptr: PoolPtr,
}

zangfx_impl_handle! { EmulatedHeapAlloc, handles::HeapAlloc }

unsafe impl Send for EmulatedHeapAlloc {}
unsafe impl Sync for EmulatedHeapAlloc {}

/// Emulated implementation of `Heap` for Metal. Does not use `MTLHeap` and
/// allocates resources from `MTLDevice` directly.
///
/// Host-visible heap is superseded by `BufferHeap` and therefore this type of
/// heap is **no longer** created by `HeapBuilder`.
///
/// Binding `MTLImage`s is not supported.
///
/// # Performance Quirks
///
/// `CmdEncoder::use_heap` runs much slower for this type of heaps because it
/// has to iterate through all allocated resources.
///
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
    pub unsafe fn new(metal_device: metal::MTLDevice, storage_mode: metal::MTLStorageMode) -> Self {
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
                    let contents_ptr = metal_buffer.contents() as *mut u8;
                    let pool_ptr = self.pool.lock().allocate(*metal_buffer);

                    let heap_alloc = EmulatedHeapAlloc {
                        contents_ptr,
                        pool_ptr,
                    };

                    handles::HeapAlloc::new(heap_alloc)
                }))
            }

            handles::ResourceRef::Image(_image) => {
                return Err(Error::with_detail(
                    ErrorKind::InvalidUsage,
                    "images cannot be bound to host-visible memory",
                ));
            }
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

    fn as_ptr(&self, alloc: &handles::HeapAlloc) -> Result<*mut u8> {
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
    let metal_buffer = unsafe { OCPtr::from_raw(allocator(size, options)) };

    if let Some(metal_buffer) = metal_buffer {
        let metal_buffer_ptr = *metal_buffer;

        // Transition the buffer to the Allocated state
        my_buffer.materialize(metal_buffer, 0, false);

        // Return `metal_buffer_ptr` for `HeapAlloc` creation
        Ok(Some(metal_buffer_ptr))
    } else {
        Ok(None)
    }
}

fn bind_image<T>(
    image: &handles::Image,
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

zangfx_impl_object! { BufferHeap: heap::Heap, ::Debug }

unsafe impl Send for BufferHeap {}
unsafe impl Sync for BufferHeap {}

#[derive(Debug)]
struct BufferHeapData {
    tlsf: SysTlsf<u32>,
    pool: Pool<Option<SysTlsfRegion>>,
}

/// Implementation of `HeapAlloc` for Metal. To be used with [`BufferHeap`].
///
/// [`BufferHeap`]: BufferHeap
#[derive(Debug, Clone)]
pub struct BufferHeapAlloc {
    /// The pointer to the resource's contents.
    contents_ptr: *mut u8,

    /// Associates this `BufferHeapAlloc` with an element of
    /// `BufferHeapData::pool`.
    pool_ptr: PoolPtr,
}

zangfx_impl_handle! { BufferHeapAlloc, handles::HeapAlloc }

unsafe impl Send for BufferHeapAlloc {}
unsafe impl Sync for BufferHeapAlloc {}

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
    fn bind(&self, obj: handles::ResourceRef) -> Result<Option<handles::HeapAlloc>> {
        match obj {
            handles::ResourceRef::Buffer(buffer) => {
                let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
                let memory_req = my_buffer.memory_req(self.metal_buffer.device());

                let contents_ptr = self.metal_buffer.contents() as *mut u8;
                let mut data = self.data.lock();

                // Allocate the region
                if memory_req.size >= 0x8000_0000 {
                    // Does not fit in 32 bits
                    return Ok(None);
                }
                data.pool.reserve(1);
                let result = data.tlsf
                    .alloc_aligned(memory_req.size as u32, memory_req.align as u32);

                if let Some((region, offset)) = result {
                    let pool_ptr = data.pool.allocate(Some(region));

                    // Transition the buffer to the Allocated state
                    my_buffer.materialize(self.metal_buffer.clone(), offset as u64, true);

                    let contents_ptr = contents_ptr.wrapping_offset(offset as isize);

                    Ok(Some(
                        BufferHeapAlloc {
                            contents_ptr,
                            pool_ptr,
                        }.into(),
                    ))
                } else {
                    Ok(None)
                }
            }

            handles::ResourceRef::Image(_image) => {
                return Err(Error::with_detail(
                    ErrorKind::InvalidUsage,
                    "BufferHeap does not support binding image resources",
                ));
            }
        }
    }

    fn make_aliasable(&self, alloc: &handles::HeapAlloc) -> Result<()> {
        let my_alloc: &BufferHeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        let mut data = self.data.lock();
        let ref mut data = *data; // Enable split borrows
        let ref mut region = data.pool[my_alloc.pool_ptr];
        if let Some(region) = region.take() {
            unsafe {
                data.tlsf.dealloc_unchecked(region);
            }
        }
        Ok(())
    }

    fn unbind(&self, alloc: &handles::HeapAlloc) -> Result<()> {
        let my_alloc: &BufferHeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        let mut data = self.data.lock();
        let region = data.pool.deallocate(my_alloc.pool_ptr).unwrap();
        if let Some(region) = region {
            unsafe {
                data.tlsf.dealloc_unchecked(region);
            }
        }
        Ok(())
    }

    fn as_ptr(&self, alloc: &handles::HeapAlloc) -> Result<*mut u8> {
        let my_alloc: &BufferHeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        Ok(my_alloc.contents_ptr)
    }
}
