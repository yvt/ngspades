//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use {RefEqArc, DeviceRef};

pub struct Buffer<T: DeviceRef> {
    data: RefEqArc<BufferData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Buffer<T> => data
}

#[derive(Debug)]
struct BufferData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::Buffer for Buffer<T> {}

impl<T: DeviceRef> core::Marker for Buffer<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
