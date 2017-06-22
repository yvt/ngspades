//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core::{self, Validate};
use metal;
use std::sync::Arc;
use std::cell::RefCell;

use {OCPtr, RefEqBox};
use imp::{Backend, Buffer, Image, DeviceData};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Heap {
    data: RefEqBox<HeapData>,
}

#[derive(Debug)]
struct HeapData {
    device: Arc<DeviceData>,
    storage_mode: metal::MTLStorageMode,
    label: RefCell<Option<String>>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct HeapAllocation {
    state: RefEqBox<HeapAllocationState>,
}

#[derive(Debug, PartialEq, Eq)]
enum HeapAllocationState {
    Invalid,
    Mappable(Buffer),
    Unmappable
}

unsafe impl Send for HeapData {}
unsafe impl Sync for HeapData {} // doesn't use MTLDevice's interior mutability

impl Heap {
    pub(crate) fn new(device: &Arc<DeviceData>, desc: &core::HeapDescription) -> Self {
        let storage_mode = match desc.storage_mode {
            core::StorageMode::Private => metal::MTLStorageMode::Private,
            core::StorageMode::Shared => metal::MTLStorageMode::Shared,
            core::StorageMode::Memoryless => metal::MTLStorageMode::Private,
        };
        Self { data: RefEqBox::new(HeapData {
            device: device.clone(),
            storage_mode,
            label: RefCell::new(None),
        }) }
    }
}

impl core::Marker for Heap {
    fn set_label(&self, label: Option<&str>) {
        let mut b = self.data.label.borrow_mut();
        *b = label.map(String::from);
    }
}

impl core::Heap<Backend> for Heap {
    fn make_buffer(&mut self,
                   description: &core::BufferDescription)
                   -> core::Result<Option<(Self::Allocation, Buffer)>> {
        let ref data = self.data;

        description.debug_expect_valid(Some(data.device.capabilities()), "");

        let buffer = Buffer::new(data.device.metal_device(),
            data.storage_mode, description)?;

        let heap_allocation_state = if data.storage_mode == metal::MTLStorageMode::Shared {
            HeapAllocationState::Mappable(buffer.clone())
        } else {
            HeapAllocationState::Unmappable
        };

        let heap_allocation = HeapAllocation {
            state: RefEqBox::new(heap_allocation_state),
        };

        Ok(Some((heap_allocation, buffer)))
    }
    fn make_image(&mut self,
                  description: &core::ImageDescription)
                  -> core::Result<Option<(Self::Allocation, Image)>> {
        unimplemented!()
    }
}

impl core::MappableHeap for Heap {
    type Allocation = HeapAllocation;
    type MappingInfo = ();

    /// No-op. Resources are not aliasable in macOS.
    fn make_aliasable(&mut self, _: &mut Self::Allocation) {}

    /// Removes a reference to the associated object.
    fn deallocate(&mut self, allocation: &mut Self::Allocation) {
        assert_ne!(*allocation.state, HeapAllocationState::Invalid);
        *allocation.state = HeapAllocationState::Invalid;
    }

    fn flush_memory(&mut self, _: &mut Self::Allocation,
        _: usize, _: Option<usize>) {
        // No-op.
        // (maybe do fence operation?)
    }

    fn invalidate_memory(&mut self, _: &mut Self::Allocation,
        _: usize, _: Option<usize>) {
        // No-op.
        // (maybe do fence operation?)
    }

    /// No-op.
    unsafe fn raw_unmap_memory(&mut self, _: Self::MappingInfo) {}

    /// Returns a pointer to the object data.
    unsafe fn raw_map_memory(&mut self,
                             allocation: &mut Self::Allocation)
                             -> (*mut u8, usize, Self::MappingInfo) {
        if let HeapAllocationState::Mappable(ref buffer) = *allocation.state {
            (buffer.contents() as *mut u8, buffer.len(), ())
        } else {
            panic!("Unmappable or invalid object");
        }
    }
}
