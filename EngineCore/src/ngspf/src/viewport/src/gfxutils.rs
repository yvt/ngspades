//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
// (excerpted from `ngsgfx/examples/common/utils.rs`)
use std::collections::VecDeque;

use self::gfx::Result;
use iterpool::{intrusive_list, Pool, PoolPtr};
use zangfx::base as gfx;

#[derive(Debug)]
pub struct HeapSet {
    device: gfx::DeviceRef,
    heaps: Pool<Heap>,
    memory_type: gfx::MemoryType,
    dynamic_heap_list: intrusive_list::ListHead,
    dynamic_heap_size: gfx::DeviceSize,
}

#[derive(Debug, Clone, Copy)]
pub struct HeapSetAlloc {
    /// A pointer to an item in `HeapSet::heaps`.
    heap_ptr: PoolPtr,
}

#[derive(Debug)]
struct Heap {
    gfx_heap: gfx::HeapRef,
    /// The number of allocations in this heap.
    use_count: usize,
    /// Is this heap dynamic?
    dynamic: bool,

    dynamic_heap_link: Option<intrusive_list::Link>,
}

impl HeapSet {
    pub fn new(device: &gfx::DeviceRef, memory_type: gfx::MemoryType) -> Self {
        Self {
            device: device.clone(),
            heaps: Pool::new(),
            memory_type,
            dynamic_heap_list: intrusive_list::ListHead::new(),
            dynamic_heap_size: 8u64 << 20,
        }
    }

    pub fn unbind(&mut self, alloc: &HeapSetAlloc, resource: gfx::ResourceRef) -> Result<()> {
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
                    heap.gfx_heap.make_aliasable(resource)?;
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
        let size = resource_ref.get_memory_req()?.size;

        if size > self.dynamic_heap_size / 2 {
            // Too big to fit in a dynamic heap
            return Ok(self
                .bind_multi(Some(resource_ref).into_iter())?
                .pop()
                .unwrap());
        }

        // Try to squeeze it into an existing dynamic heap
        let mut result = None;

        for (heap_ptr, heap) in self
            .dynamic_heap_list
            .accessor_mut(&mut self.heaps, |x| &mut x.dynamic_heap_link)
            .iter_mut()
        {
            let success = heap.gfx_heap.bind(resource_ref)?;
            if success {
                // The allocation was successful
                heap.use_count += 1;
                result = Some(heap_ptr);
                break;
            }
        }

        if let Some(heap_ptr) = result {
            // Move the heap to the front of the list
            let mut list = self
                .dynamic_heap_list
                .accessor_mut(&mut self.heaps, |x| &mut x.dynamic_heap_link);
            list.remove(heap_ptr);
            list.push_front(heap_ptr);
            return Ok(HeapSetAlloc { heap_ptr });
        }

        // Create a new heap
        let gfx_heap = self
            .device
            .build_dynamic_heap()
            .memory_type(self.memory_type)
            .size(self.dynamic_heap_size)
            .build()?;
        let success = gfx_heap.bind(resource_ref)?;
        assert!(success);
        let heap_ptr = self.heaps.allocate(Heap {
            gfx_heap,
            use_count: 1,
            dynamic: true,
            dynamic_heap_link: None,
        });
        self.dynamic_heap_list
            .accessor_mut(&mut self.heaps, |x| &mut x.dynamic_heap_link)
            .push_front(heap_ptr);

        Ok(HeapSetAlloc { heap_ptr })
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
        let mut num_resources = 0;
        builder.memory_type(self.memory_type);
        for resource in resources.clone() {
            builder.bind(resource);
            num_resources += 1;
        }
        let gfx_heap = builder.build()?;

        let heap_ptr = self.heaps.allocate(Heap {
            gfx_heap,
            use_count: num_resources,
            dynamic: false,
            dynamic_heap_link: None,
        });

        // FIXME: wtf
        let allocs = vec![HeapSetAlloc { heap_ptr }; num_resources];

        Ok(allocs)
    }
}

/// Maintains multiple `HeapSet`s to support multiple memory types.
#[derive(Debug)]
pub struct MultiHeapSet {
    device: gfx::DeviceRef,
    heap_sets: Vec<HeapSet>,
}

#[derive(Debug, Clone, Copy)]
pub struct MultiHeapSetAlloc {
    memory_type: gfx::MemoryType,
    alloc: HeapSetAlloc,
}

impl MultiHeapSet {
    pub fn new(device: &gfx::DeviceRef) -> Self {
        Self {
            device: device.clone(),
            heap_sets: Vec::new(),
        }
    }

    pub fn unbind(&mut self, alloc: &MultiHeapSetAlloc, resource: gfx::ResourceRef) -> Result<()> {
        self.heap_sets[alloc.memory_type as usize].unbind(&alloc.alloc, resource)
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
}

#[derive(Debug)]
pub struct ArgPoolSet {
    device: gfx::DeviceRef,
    arg_pools: VecDeque<gfx::ArgPoolRef>,
    table_sig: gfx::ArgTableSigRef,
}

#[derive(Debug, Clone)]
pub struct ArgPoolTable(pub gfx::ArgPoolRef, pub gfx::ArgTableRef);

impl ArgPoolSet {
    pub fn new(device: gfx::DeviceRef, table_sig: gfx::ArgTableSigRef) -> Result<Self> {
        let mut this = Self {
            device,
            arg_pools: VecDeque::new(),
            table_sig,
        };

        // Create the first `ArgPoolRef`
        let arg_pool = this.new_arg_pool()?;
        this.arg_pools.push_back(arg_pool);

        Ok(this)
    }

    pub fn table_sig(&self) -> &gfx::ArgTableSigRef {
        &self.table_sig
    }

    fn new_arg_pool(&self) -> Result<gfx::ArgPoolRef> {
        self.device
            .build_arg_pool()
            .reserve_table_sig(1024, &self.table_sig)
            .build()
    }

    pub fn new_table(&mut self) -> Result<ArgPoolTable> {
        let num_pools = self.arg_pools.len();
        for _ in 0..num_pools {
            {
                let arg_pool = self.arg_pools.front().unwrap();
                if let Some(x) = arg_pool.new_tables(1, &self.table_sig)? {
                    return Ok(ArgPoolTable(arg_pool.clone(), { x }.pop().unwrap()));
                }
            }
            let arg_pool = self.arg_pools.pop_front().unwrap();
            self.arg_pools.push_back(arg_pool);
        }

        let arg_pool = self.new_arg_pool()?;
        self.arg_pools.push_front(arg_pool);

        let arg_pool = self.arg_pools.front().unwrap();
        if let Some(x) = arg_pool.new_tables(1, &self.table_sig)? {
            return Ok(ArgPoolTable(arg_pool.clone(), { x }.pop().unwrap()));
        } else {
            unreachable!()
        }
    }
}
