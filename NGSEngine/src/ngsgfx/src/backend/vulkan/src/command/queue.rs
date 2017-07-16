//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::version::DeviceV1_0;
use std::sync::Arc;

use {DeviceRef, Backend};
use imp::{CommandBuffer, Event, Fence, DeviceData};
use super::tokenlock::Token;

pub struct CommandQueue<T: DeviceRef> {
    data: Box<CommandQueueData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for CommandQueue<T> => data
}

#[derive(Debug)]
struct CommandQueueData<T: DeviceRef> {
    device_data: Arc<DeviceData<T>>,
    pub(crate) token: Token,
}

impl<T: DeviceRef> CommandQueue<T> {
    pub(crate) fn new(device_data: &Arc<DeviceData<T>>) -> Self {
        Self {
            data: Box::new(CommandQueueData {
                device_data: device_data.clone(),
                token: Token::new(),
            }),
        }
    }

    fn device_ref(&self) -> &T {
        &self.data.device_data.device_ref
    }
}

impl<T: DeviceRef> core::CommandQueue<Backend<T>> for CommandQueue<T> {
    fn make_command_buffer(&self) -> core::Result<CommandBuffer<T>> {
        CommandBuffer::new(self.device_ref(), &self.data.device_data.cfg)
    }

    fn wait_idle(&self) {
        self.device_ref().device().device_wait_idle();
    }

    fn submit_commands(
        &self,
        buffers: &[&CommandBuffer<T>],
        event: Option<&Event<T>>,
    ) -> core::Result<()> {
        unimplemented!()
    }

    fn make_fence(&self, _: &core::FenceDescription) -> core::Result<Fence<T>> {
        Fence::new(self)
    }
}

impl<T: DeviceRef> core::Marker for CommandQueue<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
