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

/// Tracks the execution state of a command buffer.
///
/// Currently, this is implemented as a thin wrapper around `CbStateTracker`.
#[derive(Debug)]
pub struct CmdBufferAwaiter(CbStateTracker);

impl CmdBufferAwaiter {
    pub fn new(buffer: &mut gfx::CmdBuffer) -> Self {
        CmdBufferAwaiter(CbStateTracker::new(buffer))
    }

    pub fn wait_until_completed(&self) {
        self.0.wait_timeout(Duration::from_millis(1000)).unwrap();
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
