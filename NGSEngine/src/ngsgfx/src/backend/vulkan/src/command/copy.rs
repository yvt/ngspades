//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use imp::{CommandBuffer, Buffer};
use {DeviceRef, Backend};

impl<T: DeviceRef> core::CopyCommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn copy_buffer(
        &mut self,
        source: &Buffer<T>,
        source_offset: core::DeviceSize,
        destination: &Buffer<T>,
        destination_offset: core::DeviceSize,
        size: core::DeviceSize,
    ) {
        unimplemented!()
    }
}
