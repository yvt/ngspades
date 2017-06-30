//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use imp::{Backend, CommandBuffer, Buffer};

impl core::CopyCommandEncoder<Backend> for CommandBuffer {
    fn copy_buffer(
        &mut self,
        source: &Buffer,
        source_offset: usize,
        destination: &Buffer,
        destination_offset: usize,
        size: usize,
    ) {
        self.expect_copy_pipeline().copy_from_buffer_to_buffer(
            source.metal_buffer(),
            source_offset as u64,
            destination.metal_buffer(),
            destination_offset as u64,
            size as u64,
        );
    }
}
