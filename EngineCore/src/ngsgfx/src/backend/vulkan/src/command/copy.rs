//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;
use cgmath::Vector3;
use std::ops::Range;

use imp::{CommandBuffer, Buffer, Image, translate_image_layout,
          translate_image_subresource_layers, translate_image_aspect};
use {DeviceRef, Backend, AshDevice};

impl<T: DeviceRef> core::CopyCommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn fill_buffer(&mut self, destination: &Buffer<T>, range: Range<core::DeviceSize>, value: u8) {
        if self.encoder_error().is_some() {
            return;
        }

        let device: &AshDevice = self.data.device_ref.device();
        let buffer = self.expect_outside_render_pass().buffer;

        assert_eq!(
            range.start & 3,
            0,
            "range.start ({}) must be 4-byte aligned",
            range.start
        );
        assert_eq!(
            range.end & 3,
            0,
            "range.end ({}) must be 4-byte aligned",
            range.end
        );

        let data = value as u32;
        let data = data | (data << 8);
        let data = data | (data << 16);

        unsafe {
            device.fp_v1_0().cmd_fill_buffer(
                buffer,
                destination.handle(),
                range.start,
                range.end - range.start,
                data,
            );
        }
    }

    fn copy_buffer(
        &mut self,
        source: &Buffer<T>,
        source_offset: core::DeviceSize,
        destination: &Buffer<T>,
        destination_offset: core::DeviceSize,
        size: core::DeviceSize,
    ) {
        if self.encoder_error().is_some() {
            return;
        }

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

    fn copy_buffer_to_image(
        &mut self,
        source: &Buffer<T>,
        source_range: &core::BufferImageRange,
        destination: &Image<T>,
        destination_layout: core::ImageLayout,
        destination_aspect: core::ImageAspect,
        destination_subresource_range: &core::ImageSubresourceLayers,
        destination_origin: Vector3<u32>,
        size: Vector3<u32>,
    ) {
        if self.encoder_error().is_some() {
            return;
        }

        let device: &AshDevice = self.data.device_ref.device();
        let buffer = self.expect_outside_render_pass().buffer;

        unsafe {
            device.cmd_copy_buffer_to_image(
                buffer,
                source.handle(),
                destination.handle(),
                translate_image_layout(destination_layout),
                &[
                    vk::BufferImageCopy {
                        buffer_offset: source_range.offset,
                        buffer_row_length: source_range.row_stride as u32,
                        buffer_image_height: source_range.plane_stride as u32,
                        image_subresource: translate_image_subresource_layers(
                            destination_subresource_range,
                            translate_image_aspect(destination_aspect),
                        ),
                        image_offset: vk::Offset3D {
                            x: destination_origin.x as i32,
                            y: destination_origin.y as i32,
                            z: destination_origin.z as i32,
                        },
                        image_extent: vk::Extent3D {
                            width: size.x,
                            height: size.y,
                            depth: size.z,
                        },
                    },
                ],
            );
        }
    }

    fn copy_image_to_buffer(
        &mut self,
        source: &Image<T>,
        source_layout: core::ImageLayout,
        source_aspect: core::ImageAspect,
        source_subresource_range: &core::ImageSubresourceLayers,
        source_origin: Vector3<u32>,
        destination: &Buffer<T>,
        destination_range: &core::BufferImageRange,
        size: Vector3<u32>,
    ) {
        if self.encoder_error().is_some() {
            return;
        }

        let device: &AshDevice = self.data.device_ref.device();
        let buffer = self.expect_outside_render_pass().buffer;

        unsafe {
            device.fp_v1_0().cmd_copy_image_to_buffer(
                buffer,
                source.handle(),
                translate_image_layout(source_layout),
                destination.handle(),
                1,
                &vk::BufferImageCopy {
                    buffer_offset: destination_range.offset,
                    buffer_row_length: destination_range.row_stride as u32,
                    buffer_image_height: destination_range.plane_stride as u32,
                    image_subresource: translate_image_subresource_layers(
                        source_subresource_range,
                        translate_image_aspect(source_aspect),
                    ),
                    image_offset: vk::Offset3D {
                        x: source_origin.x as i32,
                        y: source_origin.y as i32,
                        z: source_origin.z as i32,
                    },
                    image_extent: vk::Extent3D {
                        width: size.x,
                        height: size.y,
                        depth: size.z,
                    },
                },
            );
        }
    }

    fn copy_image(
        &mut self,
        source: &Image<T>,
        source_layout: core::ImageLayout,
        source_subresource_range: &core::ImageSubresourceLayers,
        source_origin: Vector3<u32>,
        destination: &Image<T>,
        destination_layout: core::ImageLayout,
        destination_subresource_range: &core::ImageSubresourceLayers,
        destination_origin: Vector3<u32>,
        size: Vector3<u32>,
    ) {
        if self.encoder_error().is_some() {
            return;
        }

        let device: &AshDevice = self.data.device_ref.device();
        let buffer = self.expect_outside_render_pass().buffer;

        let src_aspect = source.info().aspect;
        let dst_aspect = source.info().aspect;

        assert_eq!(
            src_aspect,
            dst_aspect,
            "source and destination format must match"
        );

        unsafe {
            device.cmd_copy_image(
                buffer,
                source.handle(),
                translate_image_layout(source_layout),
                destination.handle(),
                translate_image_layout(destination_layout),
                &[
                    vk::ImageCopy {
                        src_subresource: translate_image_subresource_layers(
                            source_subresource_range,
                            src_aspect,
                        ),
                        src_offset: vk::Offset3D {
                            x: source_origin.x as i32,
                            y: source_origin.y as i32,
                            z: source_origin.z as i32,
                        },
                        dst_subresource: translate_image_subresource_layers(
                            destination_subresource_range,
                            src_aspect,
                        ),
                        dst_offset: vk::Offset3D {
                            x: destination_origin.x as i32,
                            y: destination_origin.y as i32,
                            z: destination_origin.z as i32,
                        },
                        extent: vk::Extent3D {
                            width: size.x,
                            height: size.y,
                            depth: size.z,
                        },
                    },
                ],
            );
        }
    }
}
