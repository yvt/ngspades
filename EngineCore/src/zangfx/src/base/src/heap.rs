//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Heap object.
use crate::resources;
use crate::{DeviceSize, MemoryType};
use crate::{Object, Result};

define_handle! {
    /// Represents a single heap allocation.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    HeapAlloc
}

/// Trait for building dynamic heap objects.
///
/// # Valid Usage
///
///  - No instance of `DynamicHeapBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::heap::DynamicHeapBuilder;
///     # fn test(device: &Device) {
///     let heap = device.build_dynamic_heap()
///         .size(1024 * 1024)
///         .memory_type(0)
///         .build()
///         .expect("Failed to create a heap.");
///     # }
///
pub trait DynamicHeapBuilder: Object {
    /// Set the heap size to `v` bytes.
    ///
    /// This property is mandatory.
    fn size(&mut self, v: DeviceSize) -> &mut DynamicHeapBuilder;

    /// Set the memory type index.
    ///
    /// This property is mandatory.
    fn memory_type(&mut self, v: MemoryType) -> &mut DynamicHeapBuilder;

    /// Build a `Heap`.
    ///
    /// # Valid Usage
    ///
    /// - All mandatory properties must have their values set before this method
    ///   is called.
    /// - The final heap size must not be zero.
    ///
    fn build(&mut self) -> Result<Box<Heap>>;
}

/// Trait for building dedicated heap objects.
///
/// Dedicated allocation is a feature that allows to describe all allocations
/// at the heap's creation time. The benefits include:
///
///  - The size of the heap will be computed automatically.
///  - Certain drivers and backends can optimize the operation of the
///    heap, for example, by utilizing Vulkan's `VK_KHR_dedicated_allocation`
///    extension.
///
/// # Valid Usage
///
///  - No instance of `DedicatedHeapBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::{Device, DedicatedHeapBuilder, Image};
///     # fn test(device: &Device, image: Image) {
///     let mut builder = device.build_dedicated_heap();
///     builder.memory_type(0);
///
///     // Pre-allocation
///     builder.prebind((&image).into());
///
///     let heap = builder.build().expect("Failed to create a heap.");
///
///     // The real allocation must done in the exactly same order
///     heap.bind((&image).into());
///     # }
///
pub trait DedicatedHeapBuilder: Object {
    /// Set the memory type index.
    ///
    /// This property is mandatory.
    fn memory_type(&mut self, v: MemoryType) -> &mut DedicatedHeapBuilder;

    /// Add a given resource to the dedicated allocation list.
    ///
    /// The return type of this method is reserved for future extensions.
    fn prebind(&mut self, obj: resources::ResourceRef);

    // FIXME: resource aliasing?

    /// Build a `Heap`.
    ///
    /// # Valid Usage
    ///
    /// - All mandatory properties must have their values set before this method
    ///   is called.
    /// - The final heap size must not be zero.
    ///
    fn build(&mut self) -> Result<Box<Heap>>;
}

/// Trait for heap objects.
///
/// The lifetime of the underlying heap object is associated with that of
/// `Heap`. Drop the `Heap` to destroy the associated heap object.
///
/// # Valid Usage
///
///  - No instance of `Heap` may outlive the originating `Device`.
///
pub trait Heap: Object {
    /// Allocate a memory region for a given resource.
    ///
    /// The resource must be in the **Prototype** state.
    ///
    /// The result is categorized as the following:
    ///
    ///  - `Ok(Some(alloc))` — The allocation was successful.
    ///  - `Ok(None)` — The allocation has failed because the heap did not have
    ///    a sufficient space.
    ///  - `Err(err)` — The allocation has failed for other reasons.
    ///
    /// # Valid Usage
    ///
    ///  - `obj` must originate from the same `Device` as the one the heap was
    ///    created from.
    ///  - If the heap is a dedicated heap, then `obj` must be one of the
    ///    resources preallocated via `DedicatedHeapBuilder::prebind`.
    ///    Furthermore, calls to `bind` must occur in the exact same order as
    ///    those to `prebind`.
    ///  - If `obj` refers to an image, this heap must not be associated with a
    ///    host-visible memory type.
    ///
    fn bind(&self, obj: resources::ResourceRef) -> Result<Option<HeapAlloc>>;

    /// Mark the allocated region available for future allocations.
    ///
    /// # Valid Usage
    ///
    ///  - `alloc` must originate from the same `Heap`.
    ///  - `alloc` must not have been deallocated yet.
    ///  - The heap must be a dynamic heap, i.e. have been created using a
    ///    `DynamicHeapBuilder`. (Dedicated heaps are not supported by this
    ///    method yet.)
    ///
    fn make_aliasable(&self, alloc: &HeapAlloc) -> Result<()>;

    /// Deallocate a memory region.
    ///
    /// The resource previously associated with the `HeapAlloc` will transition
    /// into the **Invalid** state.
    ///
    /// Note: Destroying a resource does not automatically deallocate the
    /// memory region associated with it. You must call this method explicitly.
    ///
    /// # Valid Usage
    ///
    ///  - The resource must be in the **Allocated** state.
    ///  - `alloc` must originate from the same `Heap`.
    ///  - The heap must be a dynamic heap, i.e. have been created using a
    ///    `DynamicHeapBuilder`.
    ///
    fn unbind(&self, alloc: &HeapAlloc) -> Result<()>;

    /// Get the address of the underlying storage of a resource.
    ///
    /// # Valid Usage
    ///
    ///  - The resource must be in the **Allocated** state.
    ///  - The heap's memory type must be host-visible.
    ///  - `alloc` must originate from the same `Heap`.
    ///  - `alloc` must be associated with a buffer resource.
    ///
    fn as_ptr(&self, alloc: &HeapAlloc) -> Result<*mut u8>;
}
