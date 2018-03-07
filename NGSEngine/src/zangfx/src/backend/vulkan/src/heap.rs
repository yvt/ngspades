//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Heap` and related types for Vulkan.
use ash::vk;
use ash::version::*;
use parking_lot::Mutex;
use xalloc::{SysTlsf, SysTlsfRegion};
use iterpool::{Pool, PoolPtr};

use base;
use common::{Error, ErrorKind, Result};

use device::DeviceRef;
use utils::{translate_generic_error_unwrap, translate_map_memory_error_unwrap};

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

    fn build(&mut self) -> Result<Box<base::Heap>> {
        let size = self.size
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "size"))?;
        let memory_type = self.memory_type
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "memory_type"))?;
        Heap::new(self.device, size, memory_type, size).map(|x| Box::new(x) as _)
    }
}

/// Implementation of `HeapAlloc` for Vulkan.
#[derive(Debug, Clone)]
struct HeapAlloc {
    pool_ptr: PoolPtr,
    ptr: *mut (),
}

zangfx_impl_handle! { HeapAlloc, base::HeapAlloc }

unsafe impl Sync for HeapAlloc {}
unsafe impl Send for HeapAlloc {}

/// Implementation of `Heap` for Vulkan.
#[derive(Debug)]
pub struct Heap {
    device: DeviceRef,
    ptr: *mut (),
    vk_mem: vk::DeviceMemory,
    state: Mutex<HeapState>,
}

zangfx_impl_object! { Heap: base::Heap, ::Debug }

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
            }.map_err(translate_map_memory_error_unwrap)? as *mut ();
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
    fn bind(&self, _obj: base::ResourceRef) -> Result<Option<base::HeapAlloc>> {
        unimplemented!()
    }

    fn make_aliasable(&self, alloc: &base::HeapAlloc) -> Result<()> {
        let alloc: &HeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        let mut state = self.state.lock();
        let state = &mut *state; // enable split borrowing
                                 // Keep it in the pool, but deallocate the region
        let ref mut alloc_data = state.allocations[alloc.pool_ptr];
        if let Some(region) = alloc_data.region.take() {
            unsafe {
                state.allocator.dealloc_unchecked(region);
            }
        }
        Ok(())
    }

    fn unbind(&self, alloc: &base::HeapAlloc) -> Result<()> {
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

    fn as_ptr(&self, alloc: &base::HeapAlloc) -> Result<*mut ()> {
        let alloc: &HeapAlloc = alloc.downcast_ref().expect("bad heap alloc type");
        Ok(alloc.ptr)
    }
}
