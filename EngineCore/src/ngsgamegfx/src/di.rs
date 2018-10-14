//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use injector::{prelude::*, Container};
use zangfx::base as gfx;

/// A DI container for managing objects that live at least as long as
/// `gfx::DeviceRef` do.
pub(crate) trait DeviceContainer {
    fn get_device(&self) -> &gfx::DeviceRef;
    fn get_cmd_queue_set(&self) -> &CmdQueueSet;
    fn get_main_queue(&self) -> &(gfx::CmdQueueRef, gfx::QueueFamily);
    fn get_copy_queue(&self) -> &Option<(gfx::CmdQueueRef, gfx::QueueFamily)>;
}

#[derive(Debug, Clone)]
pub(crate) struct CmdQueueSet {
    pub main_queue: (gfx::CmdQueueRef, gfx::QueueFamily),
    pub copy_queue: Option<(gfx::CmdQueueRef, gfx::QueueFamily)>,
}

impl DeviceContainer for Container {
    fn get_device(&self) -> &gfx::DeviceRef {
        &self.get_singleton().unwrap()
    }

    fn get_cmd_queue_set(&self) -> &CmdQueueSet {
        self.get_singleton().unwrap()
    }

    fn get_main_queue(&self) -> &(gfx::CmdQueueRef, gfx::QueueFamily) {
        &self.get_cmd_queue_set().main_queue
    }

    fn get_copy_queue(&self) -> &Option<(gfx::CmdQueueRef, gfx::QueueFamily)> {
        &self.get_cmd_queue_set().copy_queue
    }
}

pub(crate) fn new_device_container(
    device: gfx::DeviceRef,
    cmd_queue_set: CmdQueueSet,
) -> Container {
    let mut container = Container::new();

    container.register_singleton(device);
    container.register_singleton(cmd_queue_set);

    use crate::asyncuploader::di::DeviceContainerExt;
    container.register_async_uploader_default();

    container
}
