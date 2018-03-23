//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::mpsc;
use std::time::Duration;

use gfx;
use common::BinaryInteger;

pub use gfxut::*;

#[derive(Debug)]
pub struct CmdBufferAwaiter {
    recv: mpsc::Receiver<()>,
}

impl CmdBufferAwaiter {
    pub fn new(buffer: &mut gfx::CmdBuffer) -> Self {
        // `Sender` is not `Sync`. What.
        let (send, recv) = mpsc::sync_channel(1);

        buffer.on_complete(Box::new(move || {
            let _ = send.send(());
        }));

        Self { recv }
    }

    pub fn wait_until_completed(&self) {
        self.recv.recv_timeout(Duration::from_millis(1000)).unwrap();
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
