//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core::{self, Validate};
use std::sync::Arc;
use std::cell::RefCell;

use RefEqBox;
use imp::{Backend, Buffer, Image, DeviceData};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Heap {
    data: RefEqBox<HeapData>,
}

#[derive(Debug)]
struct HeapData {
    device: Arc<DeviceData>,
    usage: Option<core::SpecializedHeapUsage>,
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
    Unmappable,
}

unsafe impl Send for HeapData {}
unsafe impl Sync for HeapData {} // doesn't use MTLDevice's interior mutability

impl Heap {
    pub(crate) fn new_specialized(
        device: &Arc<DeviceData>,
        desc: &core::SpecializedHeapDescription,
    ) -> Self {
        Self {
            data: RefEqBox::new(HeapData {
                device: device.clone(),
                usage: Some(desc.usage),
                label: RefCell::new(None),
            }),
        }
    }

    pub(crate) fn new_universal(device: &Arc<DeviceData>) -> Self {
        Self {
            data: RefEqBox::new(HeapData {
                device: device.clone(),
                usage: None,
                label: RefCell::new(None),
            }),
        }
    }
}

impl core::Marker for Heap {
    fn set_label(&self, label: Option<&str>) {
        let mut b = self.data.label.borrow_mut();
        *b = label.map(String::from);
    }
}

impl core::Heap<Backend> for Heap {
    fn make_buffer(
        &mut self,
        description: &core::BufferDescription,
    ) -> core::Result<Option<(Self::Allocation, Buffer)>> {
        let ref data = self.data;

        if let Some(ref usage) = self.data.usage {
            debug_assert!(usage.supports_buffer(description), "wrong usage of heap");
        }

        description.debug_expect_valid(Some(data.device.capabilities()), "");

        let buffer = Buffer::new(data.device.metal_device(), description)?;

        let heap_allocation_state = if description.storage_mode == core::StorageMode::Shared {
            HeapAllocationState::Mappable(buffer.clone())
        } else {
            HeapAllocationState::Unmappable
        };

        let heap_allocation = HeapAllocation { state: RefEqBox::new(heap_allocation_state) };

        Ok(Some((heap_allocation, buffer)))
    }
    fn make_image(
        &mut self,
        description: &core::ImageDescription,
    ) -> core::Result<Option<(Self::Allocation, Image)>> {
        if let Some(ref usage) = self.data.usage {
            debug_assert!(usage.supports_image(description), "wrong usage of heap");
        }

        description.debug_expect_valid(Some(self.data.device.capabilities()), "");

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

    fn flush_memory(
        &mut self,
        _: &mut Self::Allocation,
        _: core::DeviceSize,
        _: Option<core::DeviceSize>,
    ) {
        // No-op.
    }

    fn invalidate_memory(
        &mut self,
        _: &mut Self::Allocation,
        _: core::DeviceSize,
        _: Option<core::DeviceSize>,
    ) {
        // No-op.
    }

    /// No-op.
    unsafe fn raw_unmap_memory(&mut self, _: Self::MappingInfo) {}

    /// Returns a pointer to the object data.
    unsafe fn raw_map_memory(
        &mut self,
        allocation: &mut Self::Allocation,
    ) -> (*mut u8, usize, Self::MappingInfo) {
        if let HeapAllocationState::Mappable(ref buffer) = *allocation.state {
            (buffer.contents() as *mut u8, buffer.len() as usize, ())
        } else {
            panic!("Unmappable or invalid object");
        }
    }
}
