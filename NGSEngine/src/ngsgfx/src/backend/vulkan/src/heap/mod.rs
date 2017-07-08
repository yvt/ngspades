//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use {RefEqArc, DeviceRef, Backend};
use imp::{Buffer, Image, DeviceData};

// TODO: separate heap type

pub struct Heap<T: DeviceRef> {
    data: RefEqArc<HeapData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for Heap<T> => data
}

#[derive(Debug)]
struct HeapData<T: DeviceRef> {
    device_data: DeviceData<T>,
}

impl<T: DeviceRef> core::Heap<Backend<T>> for Heap<T> {
    fn make_buffer(
        &mut self,
        description: &core::BufferDescription,
    ) -> core::Result<Option<(Self::Allocation, Buffer<T>)>> {
        unimplemented!()
    }
    fn make_image(
        &mut self,
        description: &core::ImageDescription,
    ) -> core::Result<Option<(Self::Allocation, Image<T>)>> {
        unimplemented!()
    }
}
impl<T: DeviceRef> core::MappableHeap for Heap<T> {
    type Allocation = ();
    type MappingInfo = ();
    fn make_aliasable(&mut self, allocation: &mut Self::Allocation) {
        unimplemented!()
    }
    fn deallocate(&mut self, allocation: &mut Self::Allocation) {
        unimplemented!()
    }
    unsafe fn raw_unmap_memory(&mut self, info: Self::MappingInfo) {
        unimplemented!()
    }
    unsafe fn raw_map_memory(
        &mut self,
        allocation: &mut Self::Allocation,
    ) -> (*mut u8, usize, Self::MappingInfo) {
        unimplemented!()
    }
    fn flush_memory(
        &mut self,
        allocation: &mut Self::Allocation,
        offset: core::DeviceSize,
        size: Option<core::DeviceSize>,
    ) {
        unimplemented!()
    }
    fn invalidate_memory(
        &mut self,
        allocation: &mut Self::Allocation,
        offset: core::DeviceSize,
        size: Option<core::DeviceSize>,
    ) {
        unimplemented!()
    }
}

impl<T: DeviceRef> core::Marker for Heap<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
