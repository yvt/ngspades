//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Heap object.
use Object;
use common::Result;
use handles as h;
use {DeviceSize, MemoryType};

/// Trait for building heap objects.
///
/// # Valid Usage
///
///  - No instance of `HeapBuilder` may outlive the originating `Device`.
///
/// # Allocation Provision
///
/// TODO
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::heap::HeapBuilder;
///     # fn test(device: &Device) {
///     let heap = device.build_heap()
///         .size(1024 * 1024)
///         .memory_type(0)
///         .build()
///         .expect("Failed to create a heap.");
///     # }
///
pub trait HeapBuilder: Object {
    /// Set the heap size to `v` bytes.
    ///
    /// Defaults to `0`.
    fn size(&mut self, v: DeviceSize) -> &mut HeapBuilder;

    /// Set the memory type index.
    ///
    /// This property is mandatory.
    fn memory_type(&mut self, v: MemoryType) -> &mut HeapBuilder;

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
    fn bind(&self, obj: h::ResourceRef) -> Result<Option<h::HeapAlloc>>;

    fn make_aliasable(&self, alloc: &h::HeapAlloc) -> Result<()>;

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
    ///
    fn unbind(&self, alloc: &h::HeapAlloc) -> Result<()>;

    /// Get the address of the underlying storage of a resource.
    ///
    /// # Valid Usage
    ///
    ///  - The resource must be in the **Allocated** state.
    ///  - The heap's memory type must be host-visible.
    ///  - `alloc` must originate from the same `Heap`.
    ///  - `alloc` must be associated with a buffer resource.
    ///
    fn as_ptr(&self, alloc: &h::HeapAlloc) -> Result<*mut ()>;
}
