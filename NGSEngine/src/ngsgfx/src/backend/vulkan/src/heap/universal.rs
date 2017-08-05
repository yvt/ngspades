//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::version::DeviceV1_0;
use ash::vk;
use std::sync::Arc;
use std::ptr;
use ngsgfx_common::pool::{Pool, PoolFreePtr};

use {RefEqArc, RefEqBox, DeviceRef, Backend, AshDevice, translate_map_memory_error_unwrap,
     translate_generic_error_unwrap};
use imp::{Buffer, Image, DeviceData, UnassociatedBuffer, UnassociatedImage};
use super::hunk::MemoryHunk;
use super::suballoc::{Suballocator, SuballocatorRegion};

pub struct UniversalHeap<T: DeviceRef> {
    data: RefEqBox<UniversalHeapData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for UniversalHeap<T> => data
}

#[derive(Debug)]
struct UniversalHeapData<T: DeviceRef> {
    device_data: Arc<DeviceData<T>>,
    heap_id: RefEqArc<()>,
    pools: Vec<SmallHeapPool<T>>,
}

#[derive(Debug)]
pub struct UniversalHeapAllocation<T: DeviceRef> {
    source: UniversalHeapAllocationSource<T>,
    host_visible: bool,
}

#[derive(Debug)]
enum UniversalHeapAllocationSource<T: DeviceRef> {
    SmallHeap {
        /// Specifies `SmallHeap::s_heap_id` and a region in `SmallHeap::sa`
        id: (RefEqArc<RefEqArc<()>>, SuballocatorRegion),
        memory_type: u8,
        small_heap_ptr: PoolFreePtr,
    },
    Dedicated {
        heap_id: RefEqArc<()>,
        hunk: Arc<MemoryHunk<T>>,
        size: u64,
    },
}

derive_using_field! {
    (T: DeviceRef); (PartialEq) for UniversalHeapAllocation<T> => source
}

impl<T: DeviceRef> PartialEq for UniversalHeapAllocationSource<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&UniversalHeapAllocationSource::SmallHeap { id: ref id1, .. },
             &UniversalHeapAllocationSource::SmallHeap { id: ref id2, .. }) => id1 == id2,
            (&UniversalHeapAllocationSource::Dedicated { hunk: ref hunk1, .. },
             &UniversalHeapAllocationSource::Dedicated { hunk: ref hunk2, .. }) => {
                Arc::ptr_eq(hunk1, hunk2)
            }
            _ => false,
        }
    }
}

impl<T: DeviceRef> core::Allocation for UniversalHeapAllocation<T> {
    fn is_mappable(&self) -> bool {
        self.host_visible
    }
}

impl<T: DeviceRef> core::Heap<Backend<T>> for UniversalHeap<T> {
    fn make_buffer(
        &mut self,
        description: &core::BufferDescription,
    ) -> core::Result<Option<(Self::Allocation, Buffer<T>)>> {
        let ref mut data = *self.data;

        let device_ref = data.device_data.device_ref.clone();
        let proto = UnassociatedBuffer::new(&device_ref, description)?;
        let req = proto.memory_requirements();

        let allocation = data.allocate(&req, description.storage_mode)?;
        let (hunk, offset, _) = data.get_cloned_hunk_for_allocation(&allocation);
        match proto.associate(hunk, offset) {
            Ok(obj) => Ok(Some((allocation, obj))),
            Err(err) => {
                data.deallocate(allocation);
                Err(err)
            }
        }
    }
    fn make_image(
        &mut self,
        description: &core::ImageDescription,
    ) -> core::Result<Option<(Self::Allocation, Image<T>)>> {
        let ref mut data = *self.data;

        let device_ref = data.device_data.device_ref.clone();
        let proto = UnassociatedImage::new(&device_ref, description)?;
        let req = proto.memory_requirements();

        let allocation = data.allocate(&req, description.storage_mode)?;
        let (hunk, offset, _) = data.get_cloned_hunk_for_allocation(&allocation);
        match proto.associate(hunk, offset) {
            Ok(obj) => Ok(Some((allocation, obj))),
            Err(err) => {
                data.deallocate(allocation);
                Err(err)
            }
        }
    }
}

#[derive(Debug)]
pub struct UniversalHeapMappingInfo(vk::DeviceMemory);

impl<T: DeviceRef> core::MappableHeap for UniversalHeap<T> {
    type Allocation = UniversalHeapAllocation<T>;
    type MappingInfo = UniversalHeapMappingInfo;

    fn make_aliasable(&mut self, _: &mut Self::Allocation) {
        // No-op; unsupported on universal heap
    }

    fn deallocate(&mut self, allocation: Self::Allocation) {
        self.data.deallocate(allocation)
    }

    unsafe fn raw_unmap_memory(&mut self, info: Self::MappingInfo) {
        let device: &AshDevice = self.data.device_data.device_ref.device();
        device.unmap_memory(info.0);
    }

    unsafe fn raw_map_memory(
        &mut self,
        allocation: &mut Self::Allocation,
    ) -> core::Result<(*mut u8, usize, Self::MappingInfo)> {
        let ref mut data = *self.data;
        let device: &AshDevice = data.device_data.device_ref.device();

        assert!(allocation.host_visible);

        let (buffer, offset, size) = match allocation.source {
            UniversalHeapAllocationSource::SmallHeap {
                id: (ref s_heap_id, ref region),
                small_heap_ptr,
                memory_type,
            } => {
                assert_eq!(**s_heap_id, data.heap_id, "wrong heap");

                let ref pool: SmallHeapPool<T> = data.pools[memory_type as usize];
                let ref heap: SmallHeap<T> = pool.pool[small_heap_ptr];

                (heap.hunk.handle(), region.offset(), region.size())
            }
            UniversalHeapAllocationSource::Dedicated {
                ref heap_id,
                ref hunk,
                size,
            } => {
                assert_eq!(*heap_id, data.heap_id, "wrong heap");
                (hunk.handle(), 0, size)
            }
        };

        let ptr = device.map_memory(buffer, offset, size, vk::MemoryMapFlags::empty())
            .map_err(translate_map_memory_error_unwrap)?;

        Ok((
            ptr as *mut u8,
            size as usize,
            UniversalHeapMappingInfo(buffer),
        ))
    }
}

impl<T: DeviceRef> core::Marker for UniversalHeap<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> UniversalHeap<T> {
    pub(crate) fn new(device_data: Arc<DeviceData<T>>) -> Self {
        Self {
            data: RefEqBox::new(UniversalHeapData {
                heap_id: RefEqArc::new(()),
                pools: (0..device_data.cfg.memory_types.len())
                    .map(|_| SmallHeapPool::new())
                    .collect(),
                device_data,
            }),
        }
    }
}

impl<T: DeviceRef> UniversalHeapData<T> {
    fn allocate(
        &mut self,
        req: &vk::MemoryRequirements,
        storage_mode: core::StorageMode,
    ) -> core::Result<UniversalHeapAllocation<T>> {
        let mem_type = self.device_data
            .cfg
            .storage_mode_mappings
            .map_storage_mode(storage_mode, req.memory_type_bits)
            .expect("no suitable memory type");
        let ref device_ref = self.device_data.device_ref;
        let device: &AshDevice = device_ref.device();

        let (ref mem_type_info, ref strategy) = self.device_data.cfg.memory_types[mem_type as
                                                                                      usize];

        let host_visible = mem_type_info.property_flags.subset(
            vk::MEMORY_PROPERTY_HOST_VISIBLE_BIT,
        );

        if req.size >= strategy.size_threshold {
            // Allocate directly (don't suballocate)
            let handle = unsafe {
                device.allocate_memory(
                    &vk::MemoryAllocateInfo {
                        s_type: vk::StructureType::MemoryAllocateInfo,
                        p_next: ptr::null(),
                        allocation_size: req.size,
                        memory_type_index: mem_type as u32,
                    },
                    device_ref.allocation_callbacks(),
                )
            }.map_err(translate_generic_error_unwrap)?;
            let hunk = unsafe { MemoryHunk::from_raw(device_ref, handle) };

            Ok(UniversalHeapAllocation {
                source: UniversalHeapAllocationSource::Dedicated {
                    heap_id: self.heap_id.clone(),
                    hunk: Arc::new(hunk),
                    size: req.size,
                },
                host_visible,
            })
        } else {
            let ref mut pool: SmallHeapPool<T> = self.pools[mem_type as usize];

            // Find a free heap
            if let Some(first_ptr) = pool.list.first_ptr {
                let mut ptr = first_ptr;
                loop {
                    let ref mut heap: SmallHeap<T> = pool.pool[ptr];

                    let result = heap.sa.allocate(req.size, req.alignment);
                    if let Some(region) = result {
                        // Make this the first element
                        pool.list.first_ptr = Some(ptr);
                        heap.num_allocs += 1;

                        return Ok(UniversalHeapAllocation {
                            source: UniversalHeapAllocationSource::SmallHeap {
                                id: (heap.s_heap_id.clone(), region),
                                memory_type: mem_type,
                                small_heap_ptr: ptr,
                            },
                            host_visible,
                        });
                    }

                    ptr = heap.next_ptr;
                    if ptr == first_ptr {
                        break;
                    }
                }
            }

            // Create a new heap
            pool.pool.reserve(1);

            let handle = unsafe {
                device.allocate_memory(
                    &vk::MemoryAllocateInfo {
                        s_type: vk::StructureType::MemoryAllocateInfo,
                        p_next: ptr::null(),
                        allocation_size: strategy.small_zone_size,
                        memory_type_index: mem_type as u32,
                    },
                    device_ref.allocation_callbacks(),
                )
            }.map_err(translate_generic_error_unwrap)?;
            let hunk = unsafe { MemoryHunk::from_raw(device_ref, handle) };

            let sh = SmallHeap {
                s_heap_id: RefEqArc::new(self.heap_id.clone()),
                hunk: Arc::new(hunk),
                sa: Suballocator::new(strategy.small_zone_size),
                num_allocs: 0,
                prev_ptr: PoolFreePtr::uninitialized(),
                next_ptr: PoolFreePtr::uninitialized(),
            };
            let ptr = pool.pool.allocate(sh);
            pool.list.link_front(&mut pool.pool, ptr);

            // And then suballocate in the new heap
            let ref mut heap: SmallHeap<T> = pool.pool[ptr];
            let result = heap.sa.allocate(req.size, req.alignment);
            if let Some(region) = result {
                // Make this the first element
                pool.list.first_ptr = Some(ptr);
                heap.num_allocs += 1;

                Ok(UniversalHeapAllocation {
                    source: UniversalHeapAllocationSource::SmallHeap {
                        id: (heap.s_heap_id.clone(), region),
                        memory_type: mem_type,
                        small_heap_ptr: ptr,
                    },
                    host_visible,
                })
            } else {
                unreachable!()
            }
        }
    }

    fn deallocate(&mut self, allocation: UniversalHeapAllocation<T>) {
        match allocation.source {
            UniversalHeapAllocationSource::SmallHeap {
                id: (s_heap_id, region),
                memory_type,
                small_heap_ptr,
            } => {
                assert_eq!(*s_heap_id, self.heap_id, "wrong heap");

                let ref mut pool: SmallHeapPool<T> = self.pools[memory_type as usize];
                let destroyed = {
                    let ref mut heap: SmallHeap<T> = pool.pool[small_heap_ptr];
                    heap.sa.deallocate(region);
                    heap.num_allocs -= 1;
                    heap.num_allocs == 0
                };
                if destroyed {
                    pool.list.unlink(&mut pool.pool, small_heap_ptr);
                    pool.pool.deallocate(small_heap_ptr);
                }
            }
            UniversalHeapAllocationSource::Dedicated { heap_id, .. } => {
                assert_eq!(heap_id, self.heap_id, "wrong heap");
                // No-op; automatically deallocated by drop
            }
        }
    }

    fn get_cloned_hunk_for_allocation(
        &self,
        allocation: &UniversalHeapAllocation<T>,
    ) -> (Arc<MemoryHunk<T>>, u64, u64) {
        match allocation.source {
            UniversalHeapAllocationSource::SmallHeap {
                id: (ref s_heap_id, ref region),
                small_heap_ptr,
                memory_type,
            } => {
                assert_eq!(**s_heap_id, self.heap_id, "wrong heap");

                let ref pool: SmallHeapPool<T> = self.pools[memory_type as usize];
                let ref heap: SmallHeap<T> = pool.pool[small_heap_ptr];

                (heap.hunk.clone(), region.offset(), region.size())
            }
            UniversalHeapAllocationSource::Dedicated {
                ref heap_id,
                ref hunk,
                size,
            } => {
                assert_eq!(*heap_id, self.heap_id, "wrong heap");
                (hunk.clone(), 0, size)
            }
        }
    }
}

#[derive(Debug)]
struct SmallHeapPool<T: DeviceRef> {
    pool: Pool<SmallHeap<T>>,
    list: SmallHeapList,
}

/// Circular list of `SmallHeap`s
#[derive(Debug)]
struct SmallHeapList {
    first_ptr: Option<PoolFreePtr>,
}

#[derive(Debug)]
struct SmallHeap<T: DeviceRef> {
    s_heap_id: RefEqArc<RefEqArc<()>>,
    hunk: Arc<MemoryHunk<T>>,
    sa: Suballocator,
    num_allocs: usize,

    prev_ptr: PoolFreePtr,
    next_ptr: PoolFreePtr,
}

impl<T: DeviceRef> SmallHeapPool<T> {
    fn new() -> Self {
        Self {
            pool: Pool::new(),
            list: SmallHeapList { first_ptr: None },
        }
    }
}

impl SmallHeapList {
    fn link_front<T: DeviceRef>(&mut self, pool: &mut Pool<SmallHeap<T>>, heap_ptr: PoolFreePtr) {
        if let Some(ref mut first_ptr) = self.first_ptr {
            let prev_ptr = pool[*first_ptr].prev_ptr;
            {
                let ref mut heap = pool[heap_ptr];
                heap.next_ptr = *first_ptr;
                heap.prev_ptr = prev_ptr;
            }
            pool[*first_ptr].prev_ptr = heap_ptr;
            pool[prev_ptr].next_ptr = heap_ptr;
        } else {
            let ref mut heap = pool[heap_ptr];
            heap.next_ptr = heap_ptr;
            heap.prev_ptr = heap_ptr;
        }
        self.first_ptr = Some(heap_ptr);
    }
    fn unlink<T: DeviceRef>(&mut self, pool: &mut Pool<SmallHeap<T>>, heap_ptr: PoolFreePtr) {
        let SmallHeap { prev_ptr, next_ptr, .. } = pool[heap_ptr];
        if Some(heap_ptr) == self.first_ptr {
            if next_ptr == heap_ptr {
                self.first_ptr = None;
                return;
            } else {
                self.first_ptr = Some(next_ptr);
            }
        }

        pool[prev_ptr].next_ptr = next_ptr;
        pool[next_ptr].prev_ptr = prev_ptr;
    }
}
