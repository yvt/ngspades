//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use imp::{CommandBuffer, SecondaryCommandBuffer};
use DeviceRef;

impl<T: DeviceRef> core::DebugCommandEncoder for CommandBuffer<T> {
    fn begin_debug_group(&mut self, marker: &core::DebugMarker) {
        unimplemented!()
    }

    fn end_debug_group(&mut self) {
        unimplemented!()
    }

    fn insert_debug_marker(&mut self, marker: &core::DebugMarker) {
        unimplemented!()
    }
}

impl<T: DeviceRef> core::DebugCommandEncoder for SecondaryCommandBuffer<T> {
    fn begin_debug_group(&mut self, marker: &core::DebugMarker) {
        unimplemented!()
    }

    fn end_debug_group(&mut self) {
        unimplemented!()
    }

    fn insert_debug_marker(&mut self, marker: &core::DebugMarker) {
        unimplemented!()
    }
}
