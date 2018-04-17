//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::time::Duration;

use gfx;

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
    device
        .choose_memory_type(valid_memory_types, optimal_caps, required_caps)
        .expect("Failed to find an eligible memory type.")
}
