//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::hash::Hash;
use std::fmt::Debug;
use std::cmp::{Eq, PartialEq};
use std::any::Any;
use std::marker::Send;

use super::{Result, Resources, BufferDescription, ImageDescription};

/// Represents a heap that images and buffers are allocated from.
///
/// No objects allocated from a heap shall never outlive the heap.
///
/// See the helper trait [`MappableHeap`](trait.MappableHeap.html) for functions that deal with `Allocation`s.
pub trait Heap<R: Resources>: Hash + Debug + Eq + PartialEq + Send + Any + MappableHeap {
    /// Creates a buffer and allocates a region for it.
    fn make_buffer(&mut self, description: &BufferDescription) -> Result<Option<(Self::Allocation, R::Buffer)>>;

    /// Creates an image and allocates a region for it.
    fn make_image(&mut self, description: &ImageDescription) -> Result<Option<(Self::Allocation, R::Image)>>;
}

/// Helper trait for the trait `Heap`.
pub trait MappableHeap: Hash + Debug + Eq + PartialEq + Send + Any {
    /// Represents an allocated region. Can outlive the parent `MappableHeap`.
    /// Dropping this will leak memory (useful for permanent allocations).
    type Allocation: Hash + Debug + Eq + PartialEq + Send + Any;

    /// Used to unmap a memory region.
    type MappingInfo: Debug + Send;

    /// Makes an already allocated region available for further allocations, thus allowing
    /// overlapped allocations.
    fn make_aliasable(&mut self, allocation: &mut Self::Allocation);

    /// Deallocates a region. `allocation` must orignate from the same `Heap`.
    ///
    /// Does nothing if `allocation` is already deallocated.
    fn deallocate(&mut self, allocation: &mut Self::Allocation);

    /// Unmaps a region previously mapped by `raw_map_memory`.
    /// Application developers should use `map_memory` instead of using this directly.
    unsafe fn raw_unmap_memory(&mut self, info: Self::MappingInfo);

    /// Maps a region to a host virtual memory.
    /// Application developers should use `map_memory` instead of using this directly.
    unsafe fn raw_map_memory(&mut self, allocation: &mut Self::Allocation) -> (*mut u8, usize, Self::MappingInfo);

    /// Maps a region to a host virtual memory.
    ///
    /// The heap must have been created with `StorageMode::Shared`.
    fn map_memory(&mut self, allocation: &mut Self::Allocation) -> HeapMapGuard<Self> where Self: Sized {
        let (mem, size, info) = unsafe { self.raw_map_memory(allocation) };
        HeapMapGuard {
            heap: self,
            slice: unsafe { ::std::slice::from_raw_parts_mut(mem, size) },
            info: Some(info),
        }
    }
}

/// An RAII implementation of a scoped memory map operation of `Heap`.
#[derive(Debug)]
pub struct HeapMapGuard<'a, T: MappableHeap> {
    heap: &'a mut T,
    slice: &'a mut [u8],
    info: Option<T::MappingInfo>,
}

impl<'a, T: MappableHeap> ::std::ops::Deref for HeapMapGuard<'a, T> {
    type Target = [u8];
    fn deref(&self) -> &[u8] { self.slice }
}

impl<'a, T: MappableHeap> ::std::ops::DerefMut for HeapMapGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut [u8] { self.slice }
}

impl<'a, T: MappableHeap> ::std::ops::Drop for HeapMapGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { self.heap.raw_unmap_memory(self.info.take().unwrap()); }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeapDescription {
    pub size: usize,
    pub storage_mode: StorageMode,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryRequirements {
    /// The number of bytes required for the memory allocation for the resource.
    pub size: usize,
    /// The required alignment of the resource (measured in bytes).
    pub alignment: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum StorageMode {
    /// The resource is stored in a device-local memory region.
    Private,

    /// The resource is stored in a host-accessible memory region.
    Shared,

    /// The resource is stored in an ephemeral memory region such as on-tile memory.
    /// Only accessible by the device.
    Memoryless,
}
