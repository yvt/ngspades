//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{mem, ptr};

use core;
use gfx::prelude::*;
use cgmath::Vector3;

pub struct DeviceUtils<'a, B: core::Backend> {
    device: &'a B::Device,
}

impl<'a, B: core::Backend> DeviceUtils<'a, B> {
    pub fn new(device: &'a B::Device) -> Self {
        Self { device }
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn make_preinitialized_image_no_mip<T>(
        &self,
        heap: &mut B::UniversalHeap,
        data: &[T],
        mut desc: core::ImageDescription,
        first_pipeline_stage: core::PipelineStageFlags,
        first_access_mask: core::AccessTypeFlags,
        first_layout: core::ImageLayout,
        engine: core::DeviceEngine,
    ) -> (B::Image, B::ImageView, <B::UniversalHeap as core::MappableHeap>::Allocation) {
        let device = self.device;

        let size = mem::size_of_val(data) as core::DeviceSize;
        let staging_buffer_desc = core::BufferDescription {
            usage: core::BufferUsage::TransferSource.into(),
            size,
            storage_mode: core::StorageMode::Shared,
        };
        desc.usage = desc.usage | core::ImageUsage::TransferDestination;

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
        let (allocation, image) = heap.make_image(&desc).unwrap().unwrap();

        // Add debug labels
        image.set_label(Some("preinitialized image"));
        staging_buffer.set_label(Some("staging buffer"));

        // Fill the image
        let queue = device.main_queue();
        let mut cb = queue.make_command_buffer().unwrap();
        cb.set_label(Some("staging CB to image"));
        cb.begin_encoding();
        cb.begin_copy_pass(engine);
        {
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
            cb.resource_barrier(
                core::PipelineStage::TopOfPipe.into(),
                core::AccessTypeFlags::empty(),
                core::PipelineStage::Transfer.into(),
                core::AccessType::TransferWrite.into(),
                &core::SubresourceWithLayout::Image {
                    image: &image,
                    range: Default::default(),
                    old_layout: core::ImageLayout::Undefined,
                    new_layout: core::ImageLayout::TransferDestination,
                },
            );
            cb.begin_debug_group(&core::DebugMarker::new("staging to image"));
            cb.copy_buffer_to_image(
                &staging_buffer,
                &core::BufferImageRange {
                    offset: 0,
                    row_stride: desc.extent.x as core::DeviceSize,
                    plane_stride: 0,
                },
                &image,
                core::ImageLayout::TransferDestination,
                core::ImageAspect::Color,
                &core::ImageSubresourceLayers {
                    mip_level: 0,
                    base_array_layer: 0,
                    num_array_layers: 1,
                },
                Vector3::new(0, 0, 0),
                desc.extent,
            );
            cb.end_debug_group();
            cb.resource_barrier(
                core::PipelineStage::Transfer.into(),
                core::AccessType::TransferWrite.into(),
                first_pipeline_stage,
                first_access_mask,
                &core::SubresourceWithLayout::Image {
                    image: &image,
                    range: Default::default(),
                    old_layout: core::ImageLayout::TransferDestination,
                    new_layout: first_layout,
                },
            );
        }
        cb.end_pass();
        cb.end_encoding().unwrap();

        queue.submit_commands(&mut [&mut cb], None).unwrap();

        cb.wait_completion().unwrap();

        heap.deallocate(staging_alloc);

        let image_view = device
            .factory()
            .make_image_view(&core::ImageViewDescription {
                image: &image,
                image_type: desc.image_type,
                format: desc.format,
                range: Default::default(),
            })
            .unwrap();

        // Phew! Done!
        (image, image_view, allocation)
    }
}
