//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! All buffer and image resources are allocated from heaps.
//!
//! There are two kinds of heaps:
//!
//!  1. **Universal heaps** - Supports allocation of any kinds of resources.
//!     Aliasing is not supported on this type of heap. Their usage is similar to how
//!     resources are created in legacy APIs, but note that individual heaps cannot be
//!     shared among multiple threads at the same time. This allows almost lock-free
//!     and performant implementation of such heaps.
//!
//!     Universal heaps expand as more spaces are needed. This means their allocation
//!     functions will never return `Ok(None)`. In cases such as out of video memory,
//!     they will return `Err(GenericError::OutOfDeviceMemory)`.
//!
//!     While it is possible to create multiple universal heaps, you are advised to
//!     keep the number of universal heaps as low as possible. On some backends they are
//!     implemented by allocating large chunks and suballocating resources from them,
//!     and with such implementations creating many universal heaps might leave much of
//!     space unoccupied yet unavailable for allocations via other heaps.
//!
//!  2. **Specialized heaps** - The usage and size of the heap must be specified at
//!     the creation time. Potentially have lower overheads compared to universal heaps.
//!     Since their sizes are fixed and they do not expand or shrink as being used,
//!     individual specialized heaps are more likely to be able to work independently
//!     of each other, leading to much higher and more predictable multi-thread performance.
//!
//!     Specialized heaps have fixed sizes and their allocation functions will return
//!     `Ok(None)` if there is no enough free space for the allocation request.
//!     `Err(_)` might also be returned on some situations.
//!
//!     The kind and properties of objects that can be created in the heap is specified
//!     at the creation time using `SpecializedHeapUsage`, which contains functions named
//!     `supports_image` and `supports_buffer` that can be used to check whether a resource
//!     can be allocated in a heap with a specific `SpecializedHeapUsage`.
//!
//!     Not all backends support this type of heap. You can check the support by reading
//!     `DeviceLimits::supports_specialized_heap`.
//!
//!
//! Memory Requirements
//! -------------------
//!
//! For specialized heaps, you have to specify the size of a heap before creation.
//! [`Factory`] provides methods (namely, [`get_buffer_memory_requirements`] and
//! [`get_image_memory_requirements`]) that can be used to retrieve the memory
//! requirements, which help you to determine the correct size of the heap.
//!
//! If you are using a heap like a stack, the following assumption holds:
//!
//!  - Let `R` be a set of resources with the same `SpecializedHeapUsage` properties.
//!  - Let `a_n` be the alignment requirement of each resources of `R`. (Must be a power of two)
//!  - Let `A` be the maximum value of `a_n`.
//!  - Let `s_n` be the size requirement of each resource in `R`.
//!  - Let `s'_n` be `ceiling(s_n / A) * A`.
//!  - You are guaranteed to be able to allocate and store `R` at the same time in any order
//!    in a heap created with the size at least or equal to the sum of `s'_n`s.
//!
//! For other use cases, it is possible that `R` cannot be allocated at the same
//! time because of memory fragmentation.
//!
//! FIXME: What is the point of having `alignment` in `MemoryRequirements` when
//! the unit of `size` is practically not specified?
//!
//! [`Factory`]: ../factory/trait.Factory.html
//! [`get_buffer_memory_requirements`]: ../factory/trait.Factory.html#tymethod.get_buffer_memory_requirements
//! [`get_image_memory_requirements`]: ../factory/trait.Factory.html#tymethod.get_image_memory_requirements
use std::fmt::Debug;
use std::any::Any;
use std::marker::Send;

use {Result, Backend, BufferDescription, ImageDescription, Validate, DeviceCapabilities, Marker,
     DeviceSize, BufferUsageFlags, ImageFlags, ImageUsageFlags, ImageFormat, ImageTiling};

/// Represents a heap that images and buffers are allocated from.
///
/// Objects allocated from a heap hold a reference to the underlying storage of the heap.
///
/// There are two kinds of heaps: universal heaps and specialized heaps.
/// See the [module-level documentation] for more.
///
/// See the helper trait [`MappableHeap`](trait.MappableHeap.html) for more functions.
///
/// [module-level documentation]: ../heap/
pub trait Heap<B: Backend>: Debug + Send + Any + MappableHeap + Marker {
    /// Creates a buffer and allocates a region for it.
    ///
    /// For specialized heaps, parameters specified via `description` must match
    /// those specified when the heap `self` was created.
    fn make_buffer(
        &mut self,
        description: &BufferDescription,
    ) -> Result<Option<(Self::Allocation, B::Buffer)>>;

    /// Creates an image and allocates a region for it.
    ///
    /// For specialized heaps, parameters specified via `description` must match
    /// those specified when the heap `self` was created.
    fn make_image(
        &mut self,
        description: &ImageDescription,
    ) -> Result<Option<(Self::Allocation, B::Image)>>;
}

/// Represents an allocated region.
pub trait Allocation: Debug + PartialEq + Send + Any {
    fn is_mappable(&self) -> bool;
}

/// Helper trait for the trait `Heap`.
pub trait MappableHeap: Debug + Send + Any {
    /// Represents an allocated region. Can outlive the parent `MappableHeap`.
    /// Dropping this will leak memory (useful for permanent allocations).
    ///
    /// You can specify this associate type in your code like this:
    /// `<B::UniversalHeap as MappableHeap>::Allocation` where `B: Backend`
    ///
    /// Equivalence relations might not be defined over `Allocation`s which
    /// were marked as aliasable by `make_aliasable`. For this reason,
    /// it is not required to implement `Eq` nor `Hash`.
    type Allocation: Allocation;

    /// Used to unmap a memory region.
    type MappingInfo: Debug + Send;

    /// Makes an already allocated region available for further allocations, thus allowing
    /// overlapped allocations.
    fn make_aliasable(&mut self, allocation: &mut Self::Allocation);

    /// Deallocates a region. `allocation` must orignate from the same `Heap`.
    fn deallocate(&mut self, allocation: Self::Allocation);

    /// Unmaps a region previously mapped by `raw_map_memory`.
    /// Application developers should use `map_memory` instead of using this directly.
    unsafe fn raw_unmap_memory(&mut self, info: Self::MappingInfo);

    /// Maps a region to a host virtual memory.
    /// Application developers should use `map_memory` instead of using this directly.
    ///
    /// Implementations must ensure the returned pointer is valid at least until
    /// `self` is dropped or `raw_unmap_memory` is called with the `MappingInfo` returned
    /// from this function.
    ///
    /// There always will be a corresponding call to `raw_unmap_memory` for every invocation of
    /// `raw_map_memory`.
    unsafe fn raw_map_memory(
        &mut self,
        allocation: &mut Self::Allocation,
    ) -> Result<(*mut u8, usize, Self::MappingInfo)>;

    /// Maps a region to a host virtual memory.
    ///
    /// - The heap must have been created with `StorageMode::Shared`.
    /// - If the allocation was done for an image, the image must have been
    ///   created with `ImageTiling::Linear`. This is due to the Metal backend's restriction.
    fn map_memory(&mut self, allocation: &mut Self::Allocation) -> Result<HeapMapGuard<Self>>
    where
        Self: Sized,
    {
        let (mem, size, info) = unsafe { self.raw_map_memory(allocation)? };
        Ok(HeapMapGuard {
            heap: self,
            slice: unsafe { ::std::slice::from_raw_parts_mut(mem, size) },
            info: Some(info),
        })
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
    fn deref(&self) -> &[u8] {
        self.slice
    }
}

impl<'a, T: MappableHeap> ::std::ops::DerefMut for HeapMapGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.slice
    }
}

impl<'a, T: MappableHeap> ::std::ops::Drop for HeapMapGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.heap.raw_unmap_memory(self.info.take().unwrap());
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpecializedHeapDescription {
    pub size: DeviceSize,
    pub usage: SpecializedHeapUsage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecializedHeapUsage {
    Buffers {
        storage_mode: StorageMode,
        usage: BufferUsageFlags,
    },
    Images {
        storage_mode: StorageMode,
        flags: ImageFlags,
        usage: ImageUsageFlags,
        format: ImageFormat,
        tiling: ImageTiling,
    },
}

impl SpecializedHeapUsage {
    // TODO: update to `Validate` style API

    pub fn supports_buffer(&self, description: &BufferDescription) -> bool {
        match self {
            &SpecializedHeapUsage::Buffers {
                storage_mode,
                usage,
            } => storage_mode == description.storage_mode && usage.contains(description.usage),
            &SpecializedHeapUsage::Images { .. } => false,
        }
    }

    pub fn supports_image(&self, description: &ImageDescription) -> bool {
        match self {
            &SpecializedHeapUsage::Images {
                storage_mode,
                flags,
                usage,
                format,
                tiling,
            } => {
                storage_mode == description.storage_mode && flags == description.flags &&
                    format == description.format && tiling == description.tiling &&
                    usage.contains(description.usage)
            }
            &SpecializedHeapUsage::Buffers { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryRequirements {
    /// The number of bytes required for the memory allocation for the resource.
    pub size: DeviceSize,
    /// The required alignment of the resource (measured in bytes).
    pub alignment: DeviceSize,
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

/// Validation errors for [`SpecializedHeapDescription`](struct.SpecializedHeapDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SpecializedHeapDescriptionValidationError {
    // TODO
}

impl Validate for SpecializedHeapDescription {
    type Error = SpecializedHeapDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        // TODO
    }
}
