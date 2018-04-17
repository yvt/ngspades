//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use imp::{CommandBuffer, SecondaryCommandBuffer};
use DeviceRef;

impl<T: DeviceRef> core::DebugCommandEncoder for CommandBuffer<T> {
    fn begin_debug_group(&mut self, _: &core::DebugMarker) {
        // TODO: implement
    }

    fn end_debug_group(&mut self) {
        // TODO: implement
    }

    fn insert_debug_marker(&mut self, _: &core::DebugMarker) {
        // TODO: implement
    }
}

impl<T: DeviceRef> core::DebugCommandEncoder for SecondaryCommandBuffer<T> {
    fn begin_debug_group(&mut self, _: &core::DebugMarker) {
        // TODO: implement
    }

    fn end_debug_group(&mut self) {
        // TODO: implement
    }

    fn insert_debug_marker(&mut self, _: &core::DebugMarker) {
        // TODO: implement
    }
}
