//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use {DeviceRef, Backend};

pub struct SecondaryCommandBuffer<T: DeviceRef> {
    data: Box<SecondaryCommandBufferData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for SecondaryCommandBuffer<T> => data
}

#[derive(Debug)]
struct SecondaryCommandBufferData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::SecondaryCommandBuffer<Backend<T>> for SecondaryCommandBuffer<T> {
    fn end_encoding(&mut self) {
        unimplemented!()
    }
}

impl<T: DeviceRef> core::Marker for SecondaryCommandBuffer<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
