//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use cocoa::foundation::NSRange;
use cgmath::Vector3;
use metal::{self, MTLSize, MTLOrigin};
use std::ops::Range;

use imp::{Backend, CommandBuffer, Buffer, Image};

impl core::CopyCommandEncoder<Backend> for CommandBuffer {
    fn fill_buffer(&mut self, destination: &Buffer, range: Range<core::DeviceSize>, value: u8) {
        if range.start >= range.end {
            return;
        }
        self.expect_copy_pipeline().fill_buffer(
            destination.metal_buffer(),
            NSRange::new(range.start, range.end - range.start),
            value,
        );
    }

    fn copy_buffer(
        &mut self,
        source: &Buffer,
        source_offset: core::DeviceSize,
        destination: &Buffer,
        destination_offset: core::DeviceSize,
        size: core::DeviceSize,
    ) {
        self.expect_copy_pipeline().copy_from_buffer_to_buffer(
            source.metal_buffer(),
            source_offset as u64,
            destination.metal_buffer(),
            destination_offset as u64,
            size as u64,
        );
    }

    fn copy_buffer_to_image(
        &mut self,
        source: &Buffer,
        source_range: &core::BufferImageRange,
        destination: &Image,
        _destination_layout: core::ImageLayout,
        destination_aspect: core::ImageAspect,
        destination_subresource_range: &core::ImageSubresourceLayers,
        destination_origin: Vector3<u32>,
        size: Vector3<u32>,
    ) {
        // TODO: `num_bytes_per_pixel` is incorrect for non-color images
        let pixel_size = destination.num_bytes_per_pixel();
        for i in 0..destination_subresource_range.num_array_layers {
            self.expect_copy_pipeline().copy_from_buffer_to_image(
                source.metal_buffer(),
                source_range.offset +
                    source_range.plane_stride * pixel_size as u64 * i as u64,
                source_range.row_stride * pixel_size as u64,
                source_range.plane_stride * pixel_size as u64,
                MTLSize {
                    width: size.x as u64,
                    height: size.y as u64,
                    depth: size.z as u64,
                },
                destination.metal_texture(),
                (destination_subresource_range.base_array_layer + i) as u64,
                destination_subresource_range.mip_level as u64,
                MTLOrigin {
                    x: destination_origin.x as u64,
                    y: destination_origin.y as u64,
                    z: destination_origin.z as u64,
                },
                match destination_aspect {
                    core::ImageAspect::Color => metal::MTLBlitOptionNone,
                    core::ImageAspect::Depth => metal::MTLBlitOptionDepthFromDepthStencil,
                    core::ImageAspect::Stencil => metal::MTLBlitOptionStencilFromDepthStencil,
                },
            );
        }
    }

    fn copy_image_to_buffer(
        &mut self,
        source: &Image,
        _source_layout: core::ImageLayout,
        source_aspect: core::ImageAspect,
        source_subresource_range: &core::ImageSubresourceLayers,
        source_origin: Vector3<u32>,
        destination: &Buffer,
        destination_range: &core::BufferImageRange,
        size: Vector3<u32>,
    ) {
        // TODO: `num_bytes_per_pixel` is incorrect for non-color images
        let pixel_size = source.num_bytes_per_pixel();
        for i in 0..source_subresource_range.num_array_layers {
            self.expect_copy_pipeline().copy_from_image_to_buffer(
                source.metal_texture(),
                (source_subresource_range.base_array_layer + i) as u64,
                source_subresource_range.mip_level as u64,
                MTLOrigin {
                    x: source_origin.x as u64,
                    y: source_origin.y as u64,
                    z: source_origin.z as u64,
                },
                MTLSize {
                    width: size.x as u64,
                    height: size.y as u64,
                    depth: size.z as u64,
                },
                destination.metal_buffer(),
                destination_range.offset +
                    destination_range.plane_stride * pixel_size as u64 *
                        i as u64,
                destination_range.row_stride * pixel_size as u64,
                destination_range.plane_stride * pixel_size as u64,
                match source_aspect {
                    core::ImageAspect::Color => metal::MTLBlitOptionNone,
                    core::ImageAspect::Depth => metal::MTLBlitOptionDepthFromDepthStencil,
                    core::ImageAspect::Stencil => metal::MTLBlitOptionStencilFromDepthStencil,
                },
            );
        }
    }

    fn copy_image(
        &mut self,
        source: &Image,
        _source_layout: core::ImageLayout,
        source_subresource_range: &core::ImageSubresourceLayers,
        source_origin: Vector3<u32>,
        destination: &Image,
        _destination_layout: core::ImageLayout,
        destination_subresource_range: &core::ImageSubresourceLayers,
        destination_origin: Vector3<u32>,
        size: Vector3<u32>,
    ) {
        assert_eq!(
            source_subresource_range.num_array_layers,
            destination_subresource_range.num_array_layers
        );
        for i in 0..source_subresource_range.num_array_layers {
            self.expect_copy_pipeline().copy_from_image_to_image(
                source.metal_texture(),
                (source_subresource_range.base_array_layer + i) as u64,
                source_subresource_range.mip_level as u64,
                MTLOrigin {
                    x: source_origin.x as u64,
                    y: source_origin.y as u64,
                    z: source_origin.z as u64,
                },
                MTLSize {
                    width: size.x as u64,
                    height: size.y as u64,
                    depth: size.z as u64,
                },
                destination.metal_texture(),
                (destination_subresource_range.base_array_layer + i) as u64,
                destination_subresource_range.mip_level as u64,
                MTLOrigin {
                    x: destination_origin.x as u64,
                    y: destination_origin.y as u64,
                    z: destination_origin.z as u64,
                },
            );
        }
    }
}
