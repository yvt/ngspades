//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use std::time::Duration;

use {RefEqArc, DeviceRef};

pub struct Event<T: DeviceRef> {
    data: RefEqArc<EventData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Event<T> => data
}

#[derive(Debug)]
struct EventData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::Event for Event<T> {
    fn reset(&self) -> core::Result<()> {
        unimplemented!()
    }
    fn wait(&self, _: Duration) -> core::Result<bool> {
        unimplemented!()
    }
}

impl<T: DeviceRef> core::Marker for Event<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}

pub struct Fence<T: DeviceRef> {
    data: RefEqArc<FenceData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Fence<T> => data
}

#[derive(Debug)]
struct FenceData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::Fence for Fence<T> {}

impl<T: DeviceRef> core::Marker for Fence<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
