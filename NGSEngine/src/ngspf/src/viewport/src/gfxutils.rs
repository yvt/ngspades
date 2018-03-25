//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
// (excerpted from `ngsgfx/examples/common/utils.rs`)
use std::sync::Arc;

use zangfx::base as gfx;
use self::gfx::Result;
use iterpool::{intrusive_list, Pool, PoolPtr};

#[derive(Debug)]
pub struct HeapSet {
    device: Arc<gfx::Device>,
    heaps: Pool<Heap>,
    memory_type: gfx::MemoryType,
    dynamic_heap_list: intrusive_list::ListHead,
    dynamic_heap_size: gfx::DeviceSize,
}

#[derive(Debug)]
pub struct HeapSetAlloc {
    /// A pointer to an item in `HeapSet::heaps`.
    heap_ptr: PoolPtr,
    /// ZanGFX heap allocation.
    alloc: gfx::HeapAlloc,
}

#[derive(Debug)]
struct Heap {
    gfx_heap: Box<gfx::Heap>,
    /// The number of allocations in this heap.
    use_count: usize,
    /// Is this heap dynamic?
    dynamic: bool,

    dynamic_heap_link: Option<intrusive_list::Link>,
}

impl HeapSet {
    pub fn new(device: &Arc<gfx::Device>, memory_type: gfx::MemoryType) -> Self {
        Self {
            device: Arc::clone(device),
            heaps: Pool::new(),
            memory_type,
            dynamic_heap_list: intrusive_list::ListHead::new(),
            dynamic_heap_size: 8u64 << 20,
        }
    }

    pub fn unbind(&mut self, alloc: &HeapSetAlloc) -> Result<()> {
        let dealloc;
        let unlink_dynamic_heap;
        {
            let heap = self.heaps.get_mut(alloc.heap_ptr).unwrap();
            heap.use_count -= 1;
            if heap.use_count == 0 {
                // This heap is empty -- destroy it
                dealloc = true;
                unlink_dynamic_heap = heap.dynamic;
            } else {
                dealloc = false;
                unlink_dynamic_heap = false;
                if heap.dynamic {
                    heap.gfx_heap.unbind(&alloc.alloc)?;
                }
            }
        }
        if unlink_dynamic_heap {
            self.dynamic_heap_list
                .accessor_mut(&mut self.heaps, |x| &mut x.dynamic_heap_link)
                .remove(alloc.heap_ptr);
        }
        if dealloc {
            self.heaps.deallocate(alloc.heap_ptr);
        }
        Ok(())
    }

    /// Bind a resource using an existing or newly created dynamic heap.
    pub fn bind_dynamic<'a, T: Into<gfx::ResourceRef<'a>>>(
        &mut self,
        resource: T,
    ) -> Result<HeapSetAlloc> {
        let resource_ref = resource.into();
        let size = self.device.get_memory_req(resource_ref)?.size;

        if size > self.dynamic_heap_size / 2 {
            // Too big to fit in a dynamic heap
            return Ok(self.bind_multi(Some(resource_ref).into_iter())?
                .pop()
                .unwrap());
        }

        // Try to squeeze it into an existing dynamic heap
        let mut result = None;

        for (heap_ptr, heap) in self.dynamic_heap_list
            .accessor(&self.heaps, |x| &x.dynamic_heap_link)
            .iter()
        {
            let alloc = heap.gfx_heap.bind(resource_ref)?;
            if let Some(alloc) = alloc {
                // The allocation was successful
                result = Some((alloc, heap_ptr));
                break;
            }
        }

        if let Some((alloc, heap_ptr)) = result {
            // Move the heap to the front of the list
            let mut list = self.dynamic_heap_list
                .accessor_mut(&mut self.heaps, |x| &mut x.dynamic_heap_link);
            list.remove(heap_ptr);
            list.push_front(heap_ptr);
            return Ok(HeapSetAlloc { heap_ptr, alloc });
        }

        // Create a new heap
        let gfx_heap = self.device
            .build_dynamic_heap()
            .memory_type(self.memory_type)
            .size(self.dynamic_heap_size)
            .build()?;
        let alloc = gfx_heap.bind(resource_ref)?.unwrap();
        let heap_ptr = self.heaps.allocate(Heap {
            gfx_heap,
            use_count: 1,
            dynamic: true,
            dynamic_heap_link: None,
        });
        self.dynamic_heap_list
            .accessor_mut(&mut self.heaps, |x| &mut x.dynamic_heap_link)
            .push_front(heap_ptr);

        Ok(HeapSetAlloc { heap_ptr, alloc })
    }

    /// Bind multiple resources.
    ///
    /// A new dedicated heap will be created to hold all the given resourecs.
    pub fn bind_multi<'a, I>(&mut self, resources: I) -> Result<Vec<HeapSetAlloc>>
    where
        I: Iterator<Item = gfx::ResourceRef<'a>> + Clone,
    {
        // Allocate a ZanGFX dedicated heap to hold all those resources
        let mut builder = self.device.build_dedicated_heap();
        builder.memory_type(self.memory_type);
        for resource in resources.clone() {
            builder.prebind(resource);
        }
        let gfx_heap = builder.build()?;

        let heap_ptr = self.heaps.allocate(Heap {
            gfx_heap,
            use_count: 0,
            dynamic: false,
            dynamic_heap_link: None,
        });

        // Suballocate resources from the heap and construct `HeapSetAlloc`s
        let allocs = {
            let ref mut heap = self.heaps[heap_ptr];
            resources
                .clone()
                .map(|resource| {
                    let alloc = heap.gfx_heap.bind(resource)?.unwrap();
                    heap.use_count += 1;
                    Ok(HeapSetAlloc { heap_ptr, alloc })
                })
                .collect::<Result<Vec<_>>>()
        };

        if allocs.is_err() {
            // Rollback the changes
            self.heaps.deallocate(heap_ptr);
        }

        allocs
    }

    pub fn as_ptr(&self, alloc: &HeapSetAlloc) -> Result<*mut u8> {
        let heap = self.heaps.get(alloc.heap_ptr).unwrap();
        heap.gfx_heap.as_ptr(&alloc.alloc)
    }
}

/// Maintains multiple `HeapSet`s to support multiple memory types.
#[derive(Debug)]
pub struct MultiHeapSet {
    device: Arc<gfx::Device>,
    heap_sets: Vec<HeapSet>,
}

#[derive(Debug)]
pub struct MultiHeapSetAlloc {
    memory_type: gfx::MemoryType,
    alloc: HeapSetAlloc,
}

impl MultiHeapSet {
    pub fn new(device: &Arc<gfx::Device>) -> Self {
        Self {
            device: Arc::clone(device),
            heap_sets: Vec::new(),
        }
    }

    pub fn unbind(&mut self, alloc: &MultiHeapSetAlloc) -> Result<()> {
        self.heap_sets[alloc.memory_type as usize].unbind(&alloc.alloc)
    }

    /// Bind a resource using an existing or newly created dynamic heap.
    pub fn bind_dynamic<'a, T: Into<gfx::ResourceRef<'a>>>(
        &mut self,
        memory_type: gfx::MemoryType,
        resource: T,
    ) -> Result<MultiHeapSetAlloc> {
        while self.heap_sets.len() < (memory_type + 1) as usize {
            let mem_type = self.heap_sets.len() as u32;
            self.heap_sets.push(HeapSet::new(&self.device, mem_type));
        }
        self.heap_sets[memory_type as usize]
            .bind_dynamic(resource)
            .map(|alloc| MultiHeapSetAlloc { alloc, memory_type })
    }

    pub fn as_ptr(&self, alloc: &MultiHeapSetAlloc) -> Result<*mut u8> {
        self.heap_sets[alloc.memory_type as usize].as_ptr(&alloc.alloc)
    }
}
