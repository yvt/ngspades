//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;

use imp::{CommandBuffer, Buffer};
use {DeviceRef, Backend, AshDevice};

impl<T: DeviceRef> core::CopyCommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn copy_buffer(
        &mut self,
        source: &Buffer<T>,
        source_offset: core::DeviceSize,
        destination: &Buffer<T>,
        destination_offset: core::DeviceSize,
        size: core::DeviceSize,
    ) {
        let device: &AshDevice = self.data.device_ref.device();
        let buffer = self.expect_outside_render_pass().buffer;

        unsafe {
            device.cmd_copy_buffer(
                buffer,
                source.handle(),
                destination.handle(),
                &[
                    vk::BufferCopy {
                        src_offset: source_offset,
                        dst_offset: destination_offset,
                        size,
                    },
                ],
            );
        }
    }
}
