//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Heap object.
use std::sync::Arc;

use crate::command::CmdQueueRef;
use crate::resources;
use crate::{DeviceSize, MemoryType};
use crate::{Object, Result};

/// The builder for dynamic heap objects.
pub type DynamicHeapBuilderRef = Box<dyn DynamicHeapBuilder>;

/// Trait for building dynamic heap objects.
///
/// # Examples
///
///     # use zangfx_base::*;
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
    fn size(&mut self, v: DeviceSize) -> &mut dyn DynamicHeapBuilder;

    /// Set the memory type index.
    ///
    /// This property is mandatory.
    fn memory_type(&mut self, v: MemoryType) -> &mut dyn DynamicHeapBuilder;

    /// Build a `Heap`.
    ///
    /// # Valid Usage
    ///
    /// - All mandatory properties must have their values set before this method
    ///   is called.
    /// - The final heap size must not be zero.
    ///
    fn build(&mut self) -> Result<HeapRef>;
}

/// THe builder for dedicated heap objects.;
pub type DedicatedHeapBuilderRef = Box<dyn DedicatedHeapBuilder>;

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
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, image: ImageRef) {
///     let mut builder = device.build_dedicated_heap();
///     builder.memory_type(0);
///
///     builder.bind((&image).into());
///
///     let heap = builder.build().expect("Failed to create a heap.");
///     # }
///
pub trait DedicatedHeapBuilder: Object {
    /// Specify the queue associated with the created heap.
    ///
    /// Defaults to the backend-specific value.
    fn queue(&mut self, queue: &CmdQueueRef) -> &mut dyn DedicatedHeapBuilder;

    /// Set the memory type index.
    ///
    /// This property is mandatory.
    fn memory_type(&mut self, v: MemoryType) -> &mut dyn DedicatedHeapBuilder;

    /// Add a given resource to the dedicated allocation list.
    ///
    /// The return type of this method is reserved for future extensions.
    fn bind(&mut self, obj: resources::ResourceRef);

    /// Enable uses of `use_heap` on the created heap.
    fn enable_use_heap(&mut self) -> &mut dyn DedicatedHeapBuilder;

    // FIXME: resource aliasing?

    /// Build a `Heap`.
    ///
    /// All resources in the dedicated allocation list are bound to the created
    /// heap and are transitioned to the Allocated state.
    ///
    /// The dedicated allocation list is cleared after a successful construction
    /// of a `Heap`.
    ///
    /// # Valid Usage
    ///
    /// - All mandatory properties must have their values set before this method
    ///   is called.
    /// - The final heap size must not be zero.
    /// - If `use_heap` is enabled (via `DedicatedHeapBuilder::enable_use_heap`),
    ///   every resource in  the dedicated allocation list must be associated
    ///   with the queue specified by [`DedicatedHeapBuilder::queue`].
    /// - Every resource in the dedicated allocation list must follow all rules
    ///   specified in the Valid Usage of `Heap::bind` (except for the one about
    ///   the heap type).
    ///
    fn build(&mut self) -> Result<HeapRef>;
}

/// A boxed handle representing a heap object.
pub type HeapRef = Arc<dyn Heap>;

/// Trait for heap objects.
///
/// Resources bound to a heap internally keeps a reference to the heap.
pub trait Heap: Object {
    /// Allocate a memory region for a given resource.
    ///
    /// The resource must be in the **Prototype** state.
    ///
    /// The result is categorized as the following:
    ///
    ///  - `Ok(true)` — The allocation was successful.
    ///  - `Ok(false)` — The allocation has failed because the heap did not have
    ///    a sufficient space.
    ///  - `Err(err)` — The allocation has failed for other reasons.
    ///
    /// # Valid Usage
    ///
    ///  - `obj` must originate from the same `Device` as the one the heap was
    ///    created from.
    ///  - `obj` must be in the Prototype state.
    ///  - `obj` must not be a proxy object.
    ///  - The heap must be a dynamic heap, i.e. have been created using a
    ///    `DynamicHeapBuilder`. (Dedicated heaps are not supported by this
    ///    method.)
    ///  - If `obj` refers to an image, this heap must not be associated with a
    ///    host-visible memory type.
    ///
    fn bind(&self, obj: resources::ResourceRef) -> Result<bool>;

    /// Mark the allocated region available for future allocations.
    ///
    /// Note: Destroying a resource does not automatically deallocate the
    /// memory region associated with it. You must call this method or delete
    /// all references to the heap.
    ///
    /// # Valid Usage
    ///
    ///  - `obj` must be bound to this heap.
    ///  - The heap must be a dynamic heap, i.e. have been created using a
    ///    `DynamicHeapBuilder`. (Dedicated heaps are not supported by this
    ///    method.)
    ///
    fn make_aliasable(&self, obj: resources::ResourceRef) -> Result<()>;
}
