//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use std::time::Duration;

use {RefEqArc, DeviceRef};
use imp;
use super::tokenlock::TokenLock;

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
