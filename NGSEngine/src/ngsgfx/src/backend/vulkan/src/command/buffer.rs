//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use {DeviceRef, Backend};

pub struct CommandBuffer<T: DeviceRef> {
    data: Box<CommandBufferData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for CommandBuffer<T> => data
}

#[derive(Debug)]
struct CommandBufferData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::CommandBuffer<Backend<T>> for CommandBuffer<T> {
    fn state(&self) -> core::CommandBufferState {
        unimplemented!()
    }
    fn wait_completion(&self) -> core::Result<()> {
        unimplemented!()
    }
}

impl<T: DeviceRef> core::Marker for CommandBuffer<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
