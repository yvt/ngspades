//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use std::collections::HashSet;

use base;

use cmd::fence::Fence;
use device::DeviceRef;

#[derive(Debug, Default)]
pub struct FenceSet {
    pub wait_fences: Vec<Fence>,
    pub signal_fences: HashSet<Fence>,
}

impl FenceSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn wait_fence(&mut self, fence: Fence) {
        if self.signal_fences.contains(&fence) {
            // Found a matching fence signaling operating in the same CB
            return;
        }
        self.wait_fences.push(fence);
    }

    pub fn signal_fence(&mut self, fence: Fence) {
        self.signal_fences.insert(fence);
    }
}

/// Objects associated with a command buffer. This type is used for the
/// following two purposes:
///
///  1. To pass objects with a command buffer to the queue scheduler.
///  2. To retain references to the objects until the exection of the command
///     buffer is done.
///
#[derive(Debug, Default)]
pub struct RefTable {
    // TODO
}

impl RefTable {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug)]
pub(super) struct CommonCmdEncoder {
    device: DeviceRef,
    vk_cmd_buffer: vk::CommandBuffer,
}

impl CommonCmdEncoder {
    pub fn new(device: DeviceRef, vk_cmd_buffer: vk::CommandBuffer) -> Self {
        Self {
            device,
            vk_cmd_buffer,
        }
    }

    pub fn begin_debug_group(&mut self, _label: &str) {
        // TODO: debug commands
    }

    pub fn end_debug_group(&mut self) {
        // TODO: debug commands
    }

    pub fn debug_marker(&mut self, _label: &str) {
        // TODO: debug commands
    }

    pub fn wait_fence(
        &mut self,
        _fence: &Fence,
        _src_stage: base::StageFlags,
        _barrier: &base::Barrier,
    ) {
        // TODO
    }

    pub fn update_fence(&mut self, _fence: &Fence, _src_stage: base::StageFlags) {
        // TODO
    }

    pub fn barrier(&mut self, _barrier: &base::Barrier) {
        // TODO
    }
}
