//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of argument table and pool for Vulkan.
use arrayvec::ArrayVec;
use ash::version::*;
use ash::vk;
use parking_lot::ReentrantMutex;
use std::sync::Arc;

use crate::device::DeviceRef;
use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_handle, zangfx_impl_object};

use super::{translate_descriptor_type, DescriptorCount};
use crate::resstate;
use crate::utils::{translate_generic_error_unwrap, QueueIdBuilder};

use super::layout::ArgTableSig;

/// Implementation of `ArgPoolBuilder` for Vulkan.
#[derive(Debug)]
pub struct ArgPoolBuilder {
    device: DeviceRef,
    queue_id: QueueIdBuilder,
    num_sets: u32,
    count: DescriptorCount,
    enable_destroy_tables: bool,
}

zangfx_impl_object! { ArgPoolBuilder: dyn base::ArgPoolBuilder, dyn (crate::Debug) }

impl ArgPoolBuilder {
    crate fn new(device: DeviceRef) -> Self {
        Self {
            device,
            queue_id: QueueIdBuilder::new(),
            count: DescriptorCount::new(),
            num_sets: 0,
            enable_destroy_tables: false,
        }
    }
}

impl base::ArgPoolBuilder for ArgPoolBuilder {
    fn queue(&mut self, queue: &base::CmdQueueRef) -> &mut dyn base::ArgPoolBuilder {
        self.queue_id.set(queue);
        self
    }

    fn reserve_table_sig(
        &mut self,
        count: usize,
        table: &base::ArgTableSigRef,
    ) -> &mut dyn base::ArgPoolBuilder {
        let our_table: &ArgTableSig = table
            .downcast_ref()
            .expect("bad argument table signature type");
        self.num_sets += count as u32;
        self.count += *our_table.desc_count() * count as u32;
        self
    }

    fn reserve_arg(&mut self, count: usize, ty: base::ArgType) -> &mut dyn base::ArgPoolBuilder {
        let dt = translate_descriptor_type(ty);
        self.count[dt] += count as u32;
        self
    }

    fn reserve_table(&mut self, count: usize) -> &mut dyn base::ArgPoolBuilder {
        self.num_sets += count as u32;
        self
    }

    fn enable_destroy_tables(&mut self) -> &mut dyn base::ArgPoolBuilder {
        self.enable_destroy_tables = true;
        self
    }

    fn build(&mut self) -> Result<base::ArgPoolRef> {
        let mut flags = vk::DescriptorPoolCreateFlags::empty();

        if self.enable_destroy_tables {
            flags |= vk::DESCRIPTOR_POOL_CREATE_FREE_DESCRIPTOR_SET_BIT;
        }

        let pool_sizes = self.count.as_pool_sizes();

        let queue_id = self.queue_id.get(&self.device);

        let info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DescriptorPoolCreateInfo,
            p_next: ::null(),
            flags,
            max_sets: self.num_sets,
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
        };

        let vk_device = self.device.vk_device();
        let vk_d_pool = unsafe { vk_device.create_descriptor_pool(&info, None) }
            .map_err(translate_generic_error_unwrap)?;
        Ok(Arc::new(ArgPool::new(
            self.device.clone(),
            queue_id,
            vk_d_pool,
        )))
    }
}

/// Implementation of `ArgPool` for Vulkan.
#[derive(Debug)]
pub struct ArgPool(ArgPoolDataRef);

#[derive(Debug)]
crate struct ArgPoolData {
    device: DeviceRef,
    vk_d_pool: vk::DescriptorPool,
    mutex: ReentrantMutex<()>,
    tracked_state: resstate::TrackedState<()>,
}

crate type ArgPoolDataRef = Arc<ArgPoolData>;

zangfx_impl_object! { ArgPool: dyn base::ArgPool, dyn (crate::Debug) }

impl ArgPool {
    fn new(device: DeviceRef, queue_id: resstate::QueueId, vk_d_pool: vk::DescriptorPool) -> Self {
        let mutex = ReentrantMutex::new(());
        let tracked_state = resstate::TrackedState::new(queue_id, ());
        ArgPool(Arc::new(ArgPoolData {
            device,
            vk_d_pool,
            mutex,
            tracked_state,
        }))
    }

    pub fn vk_descriptor_pool(&self) -> vk::DescriptorPool {
        self.0.vk_d_pool
    }

    crate fn data(&self) -> &ArgPoolDataRef {
        &self.0
    }
}

impl resstate::Resource for ArgPoolDataRef {
    type State = ();

    fn tracked_state(&self) -> &resstate::TrackedState<Self::State> {
        &self.tracked_state
    }
}

impl Drop for ArgPoolData {
    fn drop(&mut self) {
        unsafe {
            self.device
                .vk_device()
                .destroy_descriptor_pool(self.vk_d_pool, None);
        }
    }
}

impl base::ArgPool for ArgPool {
    fn new_tables(
        &self,
        count: usize,
        table: &base::ArgTableSigRef,
    ) -> Result<Option<Vec<base::ArgTableRef>>> {
        self.0.new_tables(count, table)
    }

    fn destroy_tables(&self, tables: &[&base::ArgTableRef]) -> Result<()> {
        self.0.destroy_tables(tables)
    }

    fn reset(&self) -> Result<()> {
        self.0.reset()
    }
}

impl ArgPoolData {
    fn new_tables(
        &self,
        count: usize,
        table: &base::ArgTableSigRef,
    ) -> Result<Option<Vec<base::ArgTableRef>>> {
        let _lock = self.mutex.lock();

        use std::cmp::min;
        use std::mem::replace;

        let sig: &ArgTableSig = table
            .downcast_ref()
            .expect("bad argument table signature type");

        // Allocate descriptor sets in chunk of 256 sets
        struct PartialTableSet<'a>(&'a ArgPoolData, Vec<base::ArgTableRef>);
        impl<'a> Drop for PartialTableSet<'a> {
            fn drop(&mut self) {
                // Conversion `&[T]` to `&[&T]`
                for chunk in self.1.chunks(256) {
                    let sets: ArrayVec<[_; 256]> = chunk.iter().collect();
                    // Ignore the deallocation errors
                    let _ = self.0.destroy_tables(&sets);
                }
            }
        }

        let ref device = self.device;
        let vk_d_pool = self.vk_d_pool;

        let mut result_set = PartialTableSet(self, Vec::with_capacity(count));

        let set_layout = sig.vk_descriptor_set_layout();
        let set_layouts: ArrayVec<[_; 256]> = (0..min(256, count)).map(|_| set_layout).collect();

        let mut remaining_count = count;
        while remaining_count > 0 {
            let chunk_size = min(remaining_count, 256);
            let info = vk::DescriptorSetAllocateInfo {
                s_type: vk::StructureType::DescriptorSetAllocateInfo,
                p_next: ::null(),
                descriptor_pool: vk_d_pool,
                descriptor_set_count: chunk_size as u32,
                p_set_layouts: set_layouts.as_ptr(),
            };

            match unsafe { device.vk_device().allocate_descriptor_sets(&info) } {
                Ok(desc) => {
                    // The allocation was successful
                    assert!(desc.len() >= chunk_size);
                    result_set
                        .1
                        .extend(desc.into_iter().map(|x| unsafe { ArgTable::new(x) }.into()))
                }
                Err(_) => {
                    // Vulkan 1.0.55 Specification 13.2. "Descriptor Sets"
                    // > Any returned error other than `VK_ERROR_OUT_OF_POOL_MEMORY_KHR` or
                    // > `VK_ERROR_FRAGMENTED_POOL` does not imply its usual meaning;
                    // > applications should assume that the allocation failed due to
                    // > fragmentation, and create a new descriptor pool.
                    return Ok(None);
                }
            }
            remaining_count -= chunk_size;
        }

        Ok(Some(replace(&mut result_set.1, Vec::new())))
    }

    fn destroy_tables(&self, tables: &[&base::ArgTableRef]) -> Result<()> {
        let _lock = self.mutex.lock();
        let device = self.device.vk_device();
        for chunk in tables.chunks(256) {
            let sets: ArrayVec<[_; 256]> = chunk
                .iter()
                .map(|x| {
                    let table: &ArgTable = x.downcast_ref().expect("bad argument table type");
                    table.vk_descriptor_set()
                }).collect();
            unsafe {
                device.free_descriptor_sets(self.vk_d_pool, &sets);
            }
        }
        Ok(())
    }

    fn reset(&self) -> Result<()> {
        let _lock = self.mutex.lock();
        let device = self.device.vk_device();
        unsafe {
            device.reset_descriptor_pool(self.vk_d_pool, vk::DescriptorPoolResetFlags::empty())
        }.map_err(translate_generic_error_unwrap)
    }
}

/// Implementation of `ArgTable` for Vulkan.
#[derive(Debug, Clone)]
pub struct ArgTable {
    vk_ds: vk::DescriptorSet,
}

zangfx_impl_handle! { ArgTable, base::ArgTableRef }

unsafe impl Sync for ArgTable {}
unsafe impl Send for ArgTable {}

impl ArgTable {
    /// Construct a `ArgTable` from a given `DescriptorSet`.
    ///
    /// ZanGFX does not maintain nor track the lifetime of the given
    /// `DescriptorSet` in any ways.
    pub unsafe fn new(vk_ds: vk::DescriptorSet) -> Self {
        Self { vk_ds }
    }

    pub fn vk_descriptor_set(&self) -> vk::DescriptorSet {
        self.vk_ds
    }
}
