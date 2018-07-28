//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Heap` and related types for Vulkan.
use ash::version::*;
use ash::vk;
use iterpool::{Pool, PoolPtr};
use parking_lot::Mutex;
use std::sync::Arc;
use xalloc::{SysTlsf, SysTlsfRegion};

use base;
use base::{Error, ErrorKind, Result};

use device::DeviceRef;
use utils::{get_memory_req, translate_generic_error_unwrap, translate_map_memory_error_unwrap};
use {buffer, image};

/// Implementation of `DynamicHeapBuilder` for Vulkan.
#[derive(Debug)]
pub struct DynamicHeapBuilder {
    device: DeviceRef,
    size: Option<base::DeviceSize>,
    memory_type: Option<base::MemoryType>,
}

zangfx_impl_object! { DynamicHeapBuilder: base::DynamicHeapBuilder, ::Debug }

impl DynamicHeapBuilder {
    pub(super) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            size: None,
            memory_type: None,
        }
    }
}

impl base::DynamicHeapBuilder for DynamicHeapBuilder {
    fn size(&mut self, v: base::DeviceSize) -> &mut base::DynamicHeapBuilder {
        self.size = Some(v);
        self
    }

    fn memory_type(&mut self, v: base::MemoryType) -> &mut base::DynamicHeapBuilder {
        self.memory_type = Some(v);
        self
    }

    fn build(&mut self) -> Result<base::HeapRef> {
        let size = self.size.expect("size");
        let memory_type = self.memory_type.expect("memory_type");
        Heap::new(self.device, size, memory_type, size).map(|x| Arc::new(x) as _)
    }
}

/// Implementation of `DedicatedHeapBuilder` for Vulkan.
#[derive(Debug)]
pub struct DedicatedHeapBuilder {
    device: DeviceRef,
    memory_type: Option<base::MemoryType>,
    allocs: Vec<(base::DeviceSize, base::DeviceSize)>,
    error: Option<Error>,
}

zangfx_impl_object! { DedicatedHeapBuilder: base::DedicatedHeapBuilder, ::Debug }

impl DedicatedHeapBuilder {
    pub(super) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            memory_type: None,
            allocs: Vec::new(),
            error: None,
        }
    }
}

impl base::DedicatedHeapBuilder for DedicatedHeapBuilder {
    fn queue(&mut self, queue: &base::CmdQueueRef) -> &mut base::DedicatedHeapBuilder {
        unimplemented!();
        self
    }

    fn memory_type(&mut self, v: base::MemoryType) -> &mut base::DedicatedHeapBuilder {
        self.memory_type = Some(v);
        self
    }

    fn enable_use_heap(&mut self) -> &mut base::DedicatedHeapBuilder {
        unimplemented!();
        self
    }

    fn bind(&mut self, obj: base::ResourceRef) {
        match get_memory_req(self.device.vk_device(), obj) {
            Ok(req) => self.allocs.push((req.size, req.align)),
            // Save the error and return it from `build`.
            Err(err) => self.error = Some(err),
        }
        unimplemented!()
    }

    fn build(&mut self) -> Result<base::HeapRef> {
        if let Some(error) = self.error.take() {
            // We can't return the full `Error` twice because it's not `Clone`.
            self.error = Some(Error::new(error.kind()));
            return Err(error);
        }

        let memory_type = self.memory_type.expect("memory_type");
        let mut heap_size = 0;

        // Since dedicated heaps do not support aliasing (yet), estimating the
        // required heap size is easy peasy cheesy¹.
        //
        // The `arena_size` argument is reserved for when we implement aliasing.
        // We'll need it to deterministically operate `SysTlsf`s.
        //
        // ¹ http://mlp.wikia.com/wiki/File:Pinkie_Pie_%22easy-peasy-cheesy!%22_S7E18.png
        for &(size, align) in self.allocs.iter() {
            heap_size = (heap_size + align - 1) & !(align - 1);
            heap_size += size;
        }

        Heap::new(self.device, heap_size, memory_type, heap_size).map(|x| Arc::new(x) as _)
    }
}

/// Implementation of `HeapAlloc` for Vulkan.
#[derive(Debug, Clone)]
struct HeapAlloc {
    pool_ptr: PoolPtr,
    ptr: *mut u8,
}

// zangfx_impl_handle! { HeapAlloc, base::HeapAllocRef }

unsafe impl Sync for HeapAlloc {}
unsafe impl Send for HeapAlloc {}

/// Implementation of `Heap` for Vulkan.
#[derive(Debug)]
pub struct Heap {
    device: DeviceRef,
    ptr: *mut u8,
    vk_mem: vk::DeviceMemory,
    state: Mutex<HeapState>,
}

zangfx_impl_object! { Heap: base::Heap, ::Debug }

unsafe impl Send for Heap {}
unsafe impl Sync for Heap {}

#[derive(Debug)]
struct HeapState {
    allocator: SysTlsf<base::DeviceSize>,
    allocations: Pool<HeapAllocData>,
}

#[derive(Debug)]
struct HeapAllocData {
    region: Option<SysTlsfRegion>,
}

impl Heap {
    fn new(
        device: DeviceRef,
        size: base::DeviceSize,
        ty: base::MemoryType,
        arena_size: base::DeviceSize,
    ) -> Result<Self> {
        let state = Mutex::new(HeapState {
            allocator: SysTlsf::new(arena_size),
            allocations: Pool::new(),
        });

        let vk_mem = unsafe {
            device.vk_device().allocate_memory(
                &vk::MemoryAllocateInfo {
                    s_type: vk::StructureType::MemoryAllocateInfo,
                    p_next: ::null(),
                    allocation_size: size,
                    memory_type_index: ty,
                },
                None,
            )
        }.map_err(translate_generic_error_unwrap)?;

        // Create `Heap` ASAP before any operations that possibly cause unwinding
        let mut heap = Heap {
            device,
            ptr: ::null_mut(),
            vk_mem,
            state,
        };

        // Map the host-visible memory (this might fail, which is why we built
        // `Heap` first)
        let memory_type_caps = device.caps().info.memory_types[ty as usize].caps;
        let is_host_visible = memory_type_caps.contains(base::MemoryTypeCaps::HostVisible);
        if is_host_visible {
            heap.ptr = unsafe {
                device
                    .vk_device()
                    .map_memory(heap.vk_mem, 0, size, vk::MemoryMapFlags::empty())
            }.map_err(translate_map_memory_error_unwrap)? as *mut u8;
        }

        Ok(heap)
    }

    pub fn vk_device_memory(&self) -> vk::DeviceMemory {
        self.vk_mem
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        unsafe {
            self.device.vk_device().free_memory(self.vk_mem, None);
        }
    }
}

impl base::Heap for Heap {
    fn bind(&self, obj: base::ResourceRef) -> Result<bool> {
        let vk_device = self.device.vk_device();
        let req = get_memory_req(vk_device, obj)?;

        // Start allocation...
        let mut state = self.state.lock();
        let state = &mut *state; // enable split borrowing

        // Allocate a memory region for the resource
        struct Alloc<'a>(Option<SysTlsfRegion>, &'a mut SysTlsf<base::DeviceSize>);
        impl<'a> Drop for Alloc<'a> {
            fn drop(&mut self) {
                if let Some(r) = self.0.take() {
                    unsafe {
                        self.1.dealloc_unchecked(r);
                    }
                }
            }
        }
        let (region, offset) = match state.allocator.alloc_aligned(req.size, req.align) {
            Some(allocation) => allocation,
            None => return unimplemented!(), //  Ok(None),
        };
        let mut region = Alloc(Some(region), &mut state.allocator);

        // Bind the resource to the memory region
        match obj {
            base::ResourceRef::Buffer(buffer) => {
                let our_buffer: &buffer::Buffer = buffer.downcast_ref().expect("bad buffer type");
                unsafe {
                    vk_device.bind_buffer_memory(our_buffer.vk_buffer(), self.vk_mem, offset)
                }.map_err(translate_map_memory_error_unwrap)?;
            }
            base::ResourceRef::Image(image) => {
                let our_image: &image::Image = image.downcast_ref().expect("bad image type");
                unsafe { vk_device.bind_image_memory(our_image.vk_image(), self.vk_mem, offset) }
                    .map_err(translate_map_memory_error_unwrap)?;
            }
        }

        // Insert it to the internal pool -- First we only allocate a place in
        // it, and then move `region` into it. We do it this way for an extra
        // exception safety.
        let pool_ptr = state.allocations.allocate(HeapAllocData { region: None });
        state.allocations[pool_ptr].region = Some(region.0.take().unwrap());

        // Compute the virtual memory of the allocated object
        let ptr = if self.ptr.is_null() {
            // We must not call `offset` on an invalid pointer -- it's UB
            ::null_mut()
        } else {
            unsafe { self.ptr.offset(offset as isize) }
        };

        unimplemented!()
        // Ok(Some(HeapAlloc { pool_ptr, ptr }.into()))
    }

    fn make_aliasable(&self, _obj: base::ResourceRef) -> Result<()> {
        unimplemented!()
        /* let alloc: &HeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        let mut state = self.state.lock();
        let state = &mut *state; // enable split borrowing

        // Keep it in the pool, but deallocate the region
        let ref mut alloc_data = state.allocations[alloc.pool_ptr];
        if let Some(region) = alloc_data.region.take() {
            unsafe {
                state.allocator.dealloc_unchecked(region);
            }
        }
        Ok(()) */
    }

    /* fn unbind(&self, alloc: &base::HeapAllocRef) -> Result<()> {
        let alloc: &HeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        let mut state = self.state.lock();

        // Remove it from the pool, and deallocate the region
        let mut alloc_data = state.allocations.deallocate(alloc.pool_ptr).unwrap();
        if let Some(region) = alloc_data.region.take() {
            unsafe {
                state.allocator.dealloc_unchecked(region);
            }
        }
        Ok(())
    }

    fn as_ptr(&self, alloc: &base::HeapAllocRef) -> Result<*mut u8> {
        let alloc: &HeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        Ok(alloc.ptr)
    } */
}
