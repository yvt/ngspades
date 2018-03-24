//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
// (excerpted from `ngsgfx/examples/common/utils.rs`)
use std::sync::Arc;

use zangfx::base as gfx;
use self::gfx::Result;
use iterpool::{Pool, PoolPtr};

#[derive(Debug)]
pub struct HeapSet {
    device: Arc<gfx::Device>,
    heaps: Pool<Heap>,
    memory_type: gfx::MemoryType,
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
}

impl HeapSet {
    pub fn new(device: &Arc<gfx::Device>, memory_type: gfx::MemoryType) -> Self {
        Self {
            device: Arc::clone(device),
            heaps: Pool::new(),
            memory_type,
        }
    }

    pub fn unbind(&mut self, alloc: &HeapSetAlloc) {
        let dealloc;
        {
            let heap = self.heaps.get_mut(alloc.heap_ptr).unwrap();
            heap.use_count -= 1;
            if heap.use_count == 0 {
                // This heap is empty -- destroy it
                dealloc = true;
            } else {
                // Do not call `unbind` because currently heaps managed by
                // `HeapSet` are all dynamic heaps
                dealloc = false;
            }
        }
        if dealloc {
            self.heaps.deallocate(alloc.heap_ptr);
        }
    }

    /// Bind multiple resources.
    ///
    /// A new heap will be created to hold all the given resourecs.
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
