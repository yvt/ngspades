//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use imp::{CommandBuffer, SecondaryCommandBuffer};

impl core::DebugCommandEncoder for CommandBuffer {
    fn begin_debug_group(&mut self, marker: &core::DebugMarker) {
        self.expect_command_encoder().push_debug_group(
            marker.name(),
        );
    }

    fn end_debug_group(&mut self) {
        self.expect_command_encoder().pop_debug_group();
    }

    fn insert_debug_marker(&mut self, marker: &core::DebugMarker) {
        self.expect_command_encoder().insert_debug_signpost(
            marker.name(),
        );
    }
}

impl core::DebugCommandEncoder for SecondaryCommandBuffer {
    fn begin_debug_group(&mut self, marker: &core::DebugMarker) {
        self.metal_command_encoder().push_debug_group(
            marker.name()
        );
    }

    fn end_debug_group(&mut self) {
        self.metal_command_encoder().pop_debug_group();
    }

    fn insert_debug_marker(&mut self, marker: &core::DebugMarker) {
        self.metal_command_encoder().insert_debug_signpost(
            marker.name(),
        );
    }
}
