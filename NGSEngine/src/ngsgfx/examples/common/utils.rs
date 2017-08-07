//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{mem, ptr};

use core;
use gfx::prelude::*;

pub struct DeviceUtils<'a, B: core::Backend> {
    device: &'a B::Device,
}

impl<'a, B: core::Backend> DeviceUtils<'a, B> {
    pub fn new(device: &'a B::Device) -> Self {
        Self { device }
    }

    pub fn make_preinitialized_buffer<T>(
        &self,
        heap: &mut B::UniversalHeap,
        data: &[T],
        usage: core::BufferUsageFlags,
        first_pipeline_stage: core::PipelineStageFlags,
        first_access_mask: core::AccessTypeFlags,
        engine: core::DeviceEngine,
    ) -> (B::Buffer, <B::UniversalHeap as core::MappableHeap>::Allocation) {
        let device = self.device;

        let size = mem::size_of_val(data) as core::DeviceSize;
        let staging_buffer_desc = core::BufferDescription {
            usage: core::BufferUsage::TransferSource.into(),
            size,
            storage_mode: core::StorageMode::Shared,
        };
        let storage_mode = core::StorageMode::Private;
        let buffer_desc = core::BufferDescription {
            usage: usage | core::BufferUsage::TransferDestination,
            size,
            storage_mode,
        };

        // Create a staging heap/buffer
        let (mut staging_alloc, staging_buffer) =
            heap.make_buffer(&staging_buffer_desc).unwrap().unwrap();
        {
            let mut map = heap.map_memory(&mut staging_alloc).unwrap();
            unsafe {
                ptr::copy(data.as_ptr(), map.as_mut_ptr() as *mut T, data.len());
            }
        }

        // Create a device heap/buffer
        let (allocation, buffer) = heap.make_buffer(&buffer_desc).unwrap().unwrap();

        // Add debug labels
        buffer.set_label(Some("preinitialized buffer"));
        staging_buffer.set_label(Some("staging buffer"));

        // Fill the buffer
        let queue = device.main_queue();
        let mut cb = queue.make_command_buffer().unwrap();
        cb.set_label(Some("staging CB to buffer"));
        cb.begin_encoding();
        cb.begin_copy_pass(engine);
        cb.acquire_resource(
            core::PipelineStage::Transfer.into(),
            core::AccessType::TransferRead.into(),
            core::DeviceEngine::Host,
            &core::SubresourceWithLayout::Buffer {
                buffer: &staging_buffer,
                offset: 0,
                len: size,
            },
        );
        cb.begin_debug_group(&core::DebugMarker::new("staging to buffer"));
        cb.copy_buffer(&staging_buffer, 0, &buffer, 0, size);
        cb.end_debug_group();
        cb.resource_barrier(
            core::PipelineStage::Transfer.into(),
            core::AccessType::TransferWrite.into(),
            first_pipeline_stage,
            first_access_mask,
            &core::SubresourceWithLayout::Buffer {
                buffer: &buffer,
                offset: 0,
                len: size,
            },
        );
        cb.end_pass();
        cb.end_encoding().unwrap();

        queue.submit_commands(&mut [&mut cb], None).unwrap();

        cb.wait_completion().unwrap();

        heap.deallocate(staging_alloc);

        // Phew! Done!
        (buffer, allocation)
    }
}
