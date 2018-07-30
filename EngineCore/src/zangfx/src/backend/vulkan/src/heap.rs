//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Heap` and related types for Vulkan.
use ash::version::*;
use ash::{prelude::VkResult, vk};
use parking_lot::Mutex;
use std::sync::{
    atomic::{AtomicPtr, Ordering},
    Arc,
};
use tokenlock::Token;
use xalloc::{SysTlsf, SysTlsfRegion};

use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_object};
use zangfx_common::{TokenCell, TokenCellRef};

use crate::device::DeviceRef;
use crate::utils::{translate_generic_error_unwrap, translate_map_memory_error_unwrap};
use crate::{buffer, image};

/// Implementation of `DynamicHeapBuilder` for Vulkan.
#[derive(Debug)]
pub struct DynamicHeapBuilder {
    device: DeviceRef,
    size: Option<base::DeviceSize>,
    memory_type: Option<base::MemoryType>,
}

zangfx_impl_object! { DynamicHeapBuilder: dyn base::DynamicHeapBuilder, dyn (crate::Debug) }

impl DynamicHeapBuilder {
    crate fn new(device: DeviceRef) -> Self {
        Self {
            device,
            size: None,
            memory_type: None,
        }
    }
}

impl base::DynamicHeapBuilder for DynamicHeapBuilder {
    fn size(&mut self, v: base::DeviceSize) -> &mut dyn base::DynamicHeapBuilder {
        self.size = Some(v);
        self
    }

    fn memory_type(&mut self, v: base::MemoryType) -> &mut dyn base::DynamicHeapBuilder {
        self.memory_type = Some(v);
        self
    }

    fn build(&mut self) -> Result<base::HeapRef> {
        let size = self.size.expect("size");
        let memory_type = self.memory_type.expect("memory_type");
        Heap::new(self.device.clone(), size, memory_type, size).map(|x| Arc::new(x) as _)
    }
}

/// Implementation of `DedicatedHeapBuilder` for Vulkan.
#[derive(Debug)]
pub struct DedicatedHeapBuilder {
    device: DeviceRef,
    memory_type: Option<base::MemoryType>,
    allocs: Vec<Resource>,
}

#[derive(Debug, Clone)]
enum Resource {
    Image(image::Image),
    Buffer(buffer::Buffer),
}

impl Resource {
    fn clone_from(obj: base::ResourceRef<'_>) -> Self {
        match obj {
            base::ResourceRef::Buffer(buffer) => {
                let our_buffer: &buffer::Buffer = buffer.downcast_ref().expect("bad buffer type");
                Resource::Buffer(our_buffer.clone())
            }
            base::ResourceRef::Image(image) => {
                let our_image: &image::Image = image.downcast_ref().expect("bad image type");
                Resource::Image(our_image.clone())
            }
        }
    }

    fn bindable(&self) -> &dyn Bindable {
        match self {
            Resource::Image(x) => x,
            Resource::Buffer(x) => x,
        }
    }
}

zangfx_impl_object! { DedicatedHeapBuilder: dyn base::DedicatedHeapBuilder, dyn (crate::Debug) }

impl DedicatedHeapBuilder {
    crate fn new(device: DeviceRef) -> Self {
        Self {
            device,
            memory_type: None,
            allocs: Vec::new(),
        }
    }
}

impl base::DedicatedHeapBuilder for DedicatedHeapBuilder {
    fn queue(&mut self, _queue: &base::CmdQueueRef) -> &mut dyn base::DedicatedHeapBuilder {
        unimplemented!()
    }

    fn memory_type(&mut self, v: base::MemoryType) -> &mut dyn base::DedicatedHeapBuilder {
        self.memory_type = Some(v);
        self
    }

    fn enable_use_heap(&mut self) -> &mut dyn base::DedicatedHeapBuilder {
        unimplemented!()
    }

    fn bind(&mut self, obj: base::ResourceRef<'_>) {
        self.allocs.push(Resource::clone_from(obj));
    }

    fn build(&mut self) -> Result<base::HeapRef> {
        use std::mem::replace;

        let memory_type = self.memory_type.expect("memory_type");

        let allocs = replace(&mut self.allocs, Vec::new());

        // Since dedicated heaps do not support aliasing (yet), estimating the
        // required heap size is easy peasy cheesy¹.
        //
        // The `arena_size` argument is reserved for when we implement aliasing.
        // We'll need it to deterministically operate `SysTlsf`s.
        //
        // ¹ http://mlp.wikia.com/wiki/File:Pinkie_Pie_%22easy-peasy-cheesy!%22_S7E18.png
        let mut heap_size = 0;
        for resource in allocs.iter() {
            let req = resource.bindable().memory_req();
            heap_size = (heap_size + req.align - 1) & !(req.align - 1);
            heap_size += req.size;
        }

        let mut heap = Heap::new(self.device.clone(), heap_size, memory_type, heap_size)?;

        // Bind resources
        for resource in allocs.iter() {
            let success = heap
                .state
                .get_mut()
                .bind(&heap.vulkan_memory, resource.bindable())?;
            assert!(success, "allocation has unexpectecdly failed");
        }

        Ok(Arc::new(heap))
    }
}

/// Implementation of `Heap` for Vulkan.
#[derive(Debug)]
pub struct Heap {
    vulkan_memory: Arc<VulkanMemory>,
    state: Mutex<HeapState>,
}

zangfx_impl_object! { Heap: dyn base::Heap, dyn (crate::Debug) }

#[derive(Debug)]
struct HeapState {
    allocator: SysTlsf<base::DeviceSize>,

    /// The token used to take an ownership of `HeapBindingInfo::binding`.
    token: Token,
}

/// A (kind of) smart pointer of `vk::DeviceMemory`.
#[derive(Debug)]
struct VulkanMemory {
    device: DeviceRef,
    vk_mem: vk::DeviceMemory,
    ptr: *mut u8,
}

unsafe impl Send for VulkanMemory {}
unsafe impl Sync for VulkanMemory {}

/// Describes a binding between a resource and heap. Stored on a resource.
#[derive(Debug)]
crate struct HeapBindingInfo {
    binding: TokenCell<Option<HeapBinding>>,

    /// The host-visible pointer to the contents. Only valid for host-visible
    /// buffers.
    ptr: AtomicPtr<u8>,
}

/// A part of `HeapBindingInfo` that requires a mutable borrow to a heap's
/// internal data to access.
#[derive(Debug)]
struct HeapBinding {
    vulkan_memory: Arc<VulkanMemory>,
    region: Option<SysTlsfRegion>,
}

/// A resource object that can be bound to a heap.
crate trait Bindable {
    fn memory_req(&self) -> base::MemoryReq;
    fn binding_info(&self) -> &HeapBindingInfo;

    /// Call either `bind_buffer_memory` or `bind_image_memory` depending on the
    /// resource type.
    unsafe fn bind(
        &self,
        vk_device_memory: vk::DeviceMemory,
        offset: vk::DeviceSize,
    ) -> VkResult<()>;
}

impl VulkanMemory {
    fn new(device: DeviceRef, size: base::DeviceSize, ty: base::MemoryType) -> Result<Self> {
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
        let mut vulkan_memory = VulkanMemory {
            device,
            ptr: ::null_mut(),
            vk_mem,
        };

        // Map the host-visible memory (this might fail, which is why we built
        // `vulkan_memory` first)
        let memory_type_caps = vulkan_memory.device.caps().info.memory_types[ty as usize].caps;
        let is_host_visible = memory_type_caps.contains(base::MemoryTypeCaps::HostVisible);
        if is_host_visible {
            vulkan_memory.ptr = unsafe {
                vulkan_memory.device.vk_device().map_memory(
                    vulkan_memory.vk_mem,
                    0,
                    size,
                    vk::MemoryMapFlags::empty(),
                )
            }.map_err(translate_map_memory_error_unwrap)?
                as *mut u8;
        }

        Ok(vulkan_memory)
    }

    crate fn vk_device_memory(&self) -> vk::DeviceMemory {
        self.vk_mem
    }
}

impl Drop for VulkanMemory {
    fn drop(&mut self) {
        unsafe {
            self.device.vk_device().free_memory(self.vk_mem, None);
        }
    }
}

impl HeapBindingInfo {
    crate fn new() -> Self {
        Self {
            binding: TokenCell::new(None),
            ptr: Default::default(),
        }
    }

    crate fn as_ptr(&self) -> *mut u8 {
        let ptr = self.ptr.load(Ordering::Relaxed);
        if ptr.is_null() {
            panic!("resource is not bound or not host-visible");
        }
        ptr
    }
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
            token: Token::new(),
        });

        let vulkan_memory = VulkanMemory::new(device, size, ty)?;

        let heap = Heap {
            vulkan_memory: Arc::new(vulkan_memory),
            state,
        };

        Ok(heap)
    }

    pub fn vk_device_memory(&self) -> vk::DeviceMemory {
        self.vulkan_memory.vk_device_memory()
    }
}

fn bindable_from_resource_ref(obj: base::ResourceRef<'_>) -> &dyn Bindable {
    match obj {
        base::ResourceRef::Buffer(buffer) => {
            let our_buffer: &buffer::Buffer = buffer.downcast_ref().expect("bad buffer type");
            our_buffer
        }
        base::ResourceRef::Image(image) => {
            let our_image: &image::Image = image.downcast_ref().expect("bad image type");
            our_image
        }
    }
}

impl HeapState {
    fn bind(&mut self, vulkan_memory: &Arc<VulkanMemory>, bindable: &dyn Bindable) -> Result<bool> {
        use std::mem::ManuallyDrop;

        let req = bindable.memory_req();

        // Claim an exclusive ownership of `HeapBindingInfo::binding` of the
        // resource.
        struct Binding<'a>(ManuallyDrop<TokenCellRef<'a, Option<HeapBinding>>>);
        impl<'a> Drop for Binding<'a> {
            fn drop(&mut self) {
                // Move out the contents
                let guard = unsafe { ::std::ptr::read(&*self.0) };
                if guard.is_none() {
                    // Something went wrong. Relinquish the ownership.
                    TokenCellRef::release(guard);
                }
            }
        }

        let binding_info = bindable.binding_info();
        let binding = binding_info
            .binding
            .acquire(&mut self.token)
            .expect("resource is already, or is being bound to another heap");
        let mut binding = Binding(ManuallyDrop::new(binding));

        // Allocate a memory region for the resource
        struct Alloc<'a>(Option<SysTlsfRegion>, &'a mut SysTlsf<base::DeviceSize>);
        impl<'a> Drop for Alloc<'a> {
            fn drop(&mut self) {
                if let Some(r) = self.0.take() {
                    // Something went wrong. Undo the allocation.
                    unsafe { self.1.dealloc_unchecked(r) };
                }
            }
        }

        let (region, offset) = match self.allocator.alloc_aligned(req.size, req.align) {
            Some(allocation) => allocation,
            None => return Ok(false),
        };
        let mut region = Alloc(Some(region), &mut self.allocator);

        // Bind the resource to the memory region
        // This is an irreversible operation.
        unsafe { bindable.bind(vulkan_memory.vk_device_memory(), offset) }
            .map_err(translate_map_memory_error_unwrap)?;

        // Store the binding info to the resource
        **binding.0 = Some(HeapBinding {
            vulkan_memory: Arc::clone(vulkan_memory),
            region: Some(region.0.take().unwrap()),
        });

        // Compute the virtual memory of the allocated object
        let memory_ptr = vulkan_memory.ptr;
        let ptr = if memory_ptr.is_null() {
            ::null_mut()
        } else {
            memory_ptr.wrapping_offset(offset as isize)
        };

        binding_info.ptr.store(ptr, Ordering::Relaxed);

        Ok(true)
    }

    fn make_aliasable(&mut self, bindable: &dyn Bindable) -> Result<()> {
        let binding_info = bindable.binding_info();

        let mut binding_maybe = binding_info
            .binding
            .borrow(&mut self.token)
            .expect("resource is not bound to this heap");

        let binding = binding_maybe.as_mut().unwrap();

        if let Some(region) = binding.region.take() {
            unsafe {
                self.allocator.dealloc_unchecked(region);
            }
        }

        Ok(())
    }
}

impl base::Heap for Heap {
    fn bind(&self, obj: base::ResourceRef<'_>) -> Result<bool> {
        let bindable = bindable_from_resource_ref(obj);

        let mut state = self.state.lock();

        state.bind(&self.vulkan_memory, bindable)
    }

    fn make_aliasable(&self, obj: base::ResourceRef<'_>) -> Result<()> {
        let bindable = bindable_from_resource_ref(obj);

        let mut state = self.state.lock();

        state.make_aliasable(bindable)
    }
}
