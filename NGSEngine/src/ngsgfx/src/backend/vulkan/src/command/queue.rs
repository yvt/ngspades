//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use {DeviceRef, Backend};
use imp::{CommandBuffer, Event, Fence};

pub struct CommandQueue<T: DeviceRef> {
    data: Box<CommandQueueData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for CommandQueue<T> => data
}

#[derive(Debug)]
struct CommandQueueData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::CommandQueue<Backend<T>> for CommandQueue<T> {
    fn make_command_buffer(&self) -> core::Result<CommandBuffer<T>> {
        unimplemented!()
    }

    fn wait_idle(&self) {
        unimplemented!()
    }

    fn submit_commands(
        &self,
        buffers: &[&CommandBuffer<T>],
        event: Option<&Event<T>>,
    ) -> core::Result<()> {
        unimplemented!()
    }

    fn make_fence(&self, description: &core::FenceDescription) -> core::Result<Fence<T>> {
        unimplemented!()
    }
}

impl<T: DeviceRef> core::Marker for CommandQueue<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
