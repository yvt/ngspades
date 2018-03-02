//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::mpsc;
use std::time::Duration;
use std::borrow::Borrow;
use std::ops::Deref;

use gfx;
use common::BinaryInteger;

#[derive(Debug)]
pub struct CmdBufferAwaiter {
    recv: mpsc::Receiver<()>,
}

impl CmdBufferAwaiter {
    pub fn new(buffer: &mut gfx::CmdBuffer) -> Self {
        let (send, recv) = mpsc::channel();

        buffer.on_complete(Box::new(move || {
            let _ = send.send(());
        }));

        Self { recv }
    }

    pub fn wait_until_completed(&self) {
        self.recv.recv_timeout(Duration::from_millis(1000)).unwrap();
    }
}

#[derive(Debug)]
pub struct UniqueBuffer<D: Borrow<gfx::Device>> {
    device: D,
    buffer: gfx::Buffer,
}

impl<D: Borrow<gfx::Device>> UniqueBuffer<D> {
    pub fn new(device: D, buffer: gfx::Buffer) -> Self {
        Self { device, buffer }
    }
}

impl<D: Borrow<gfx::Device>> Deref for UniqueBuffer<D> {
    type Target = gfx::Buffer;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<D: Borrow<gfx::Device>> Drop for UniqueBuffer<D> {
    fn drop(&mut self) {
        self.device.borrow().destroy_buffer(&self.buffer).unwrap();
    }
}

pub fn choose_memory_type(
    device: &gfx::Device,
    valid_memory_types: u32,
    optimal_caps: gfx::MemoryTypeCapsFlags,
    required_caps: gfx::MemoryTypeCapsFlags,
) -> gfx::MemoryType {
    // Based on the algorithm shown in Vulkan specification 1.0
    // "10.2. Device Memory".
    let memory_types = device.caps().memory_types();

    for i in valid_memory_types.one_digits() {
        if memory_types[i as usize].caps.contains(optimal_caps) {
            return i;
        }
    }

    for i in valid_memory_types.one_digits() {
        if memory_types[i as usize].caps.contains(required_caps) {
            return i;
        }
    }

    panic!("Failed to find an eligible memory type.");
}
