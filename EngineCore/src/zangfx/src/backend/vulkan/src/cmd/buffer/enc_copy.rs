//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::version::*;
use ash::vk;
use std::ops::Range;

use zangfx_base as base;
use zangfx_common::IntoWithPad;

use crate::buffer::Buffer;
use crate::image::Image;
use crate::utils::translate_image_aspect;

use super::CmdBufferData;

impl base::CopyCmdEncoder for CmdBufferData {
    fn fill_buffer(&mut self, buffer: &base::BufferRef, range: Range<base::DeviceSize>, value: u8) {
        if range.start >= range.end {
            return;
        }
        let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        let vk_device = self.device.vk_device();

        self.ref_table.insert_buffer(my_buffer);

        let data = (value as u32) * 0x1010101;

        unsafe {
            vk_device.cmd_fill_buffer(
                self.vk_cmd_buffer(),
                my_buffer.vk_buffer(),
                range.start,
                range.end - range.start,
                data,
            );
        }
    }

    fn copy_buffer(
        &mut self,
        src: &base::BufferRef,
        src_offset: base::DeviceSize,
        dst: &base::BufferRef,
        dst_offset: base::DeviceSize,
        size: base::DeviceSize,
    ) {
        let my_src: &Buffer = src.downcast_ref().expect("bad buffer type");
        let my_dst: &Buffer = dst.downcast_ref().expect("bad buffer type");
        let vk_device = self.device.vk_device();

        self.ref_table.insert_buffer(my_src);
        self.ref_table.insert_buffer(my_dst);

        unsafe {
            vk_device.cmd_copy_buffer(
                self.vk_cmd_buffer(),
                my_src.vk_buffer(),
                my_dst.vk_buffer(),
                &[vk::BufferCopy {
                    src_offset,
                    dst_offset,
                    size,
                }],
            );
        }
    }

    // TODO: automatic image layout transitions

    fn copy_buffer_to_image(
        &mut self,
        src: &base::BufferRef,
        src_range: &base::BufferImageRange,
        dst: &base::ImageRef,
        dst_aspect: base::ImageAspect,
        dst_range: &base::ImageLayerRange,
        dst_origin: &[u32],
        size: &[u32],
    ) {
        let my_src: &Buffer = src.downcast_ref().expect("bad source buffer type");
        let my_dst: &Image = dst.downcast_ref().expect("bad destination image type");

        self.ref_table.insert_buffer(my_src);
        // TODO: Ref-count image

        let dst_origin: [u32; 3] = dst_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        let vk_device = self.device.vk_device();

        unsafe {
            vk_device.cmd_copy_buffer_to_image(
                self.vk_cmd_buffer(),
                my_src.vk_buffer(),
                my_dst.vk_image(),
                my_dst.translate_layout(base::ImageLayout::CopyWrite),
                &[vk::BufferImageCopy {
                    buffer_offset: src_range.offset,
                    buffer_row_length: src_range.row_stride as u32,
                    buffer_image_height: src_range.plane_stride as u32,
                    image_subresource: my_dst.resolve_vk_subresource_layers(
                        dst_range,
                        translate_image_aspect(dst_aspect),
                    ),
                    image_offset: vk::Offset3D {
                        x: dst_origin[0] as i32,
                        y: dst_origin[1] as i32,
                        z: dst_origin[2] as i32,
                    },
                    image_extent: vk::Extent3D {
                        width: size[0],
                        height: size[1],
                        depth: size[2],
                    },
                }],
            );
        }
    }

    fn copy_image_to_buffer(
        &mut self,
        src: &base::ImageRef,
        src_aspect: base::ImageAspect,
        src_range: &base::ImageLayerRange,
        src_origin: &[u32],
        dst: &base::BufferRef,
        dst_range: &base::BufferImageRange,
        size: &[u32],
    ) {
        let my_src: &Image = src.downcast_ref().expect("bad source image type");
        let my_dst: &Buffer = dst.downcast_ref().expect("bad destination buffer type");

        self.ref_table.insert_buffer(my_dst);
        // TODO: Ref-count image

        let src_origin: [u32; 3] = src_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        let vk_device = self.device.vk_device();

        unsafe {
            vk_device.fp_v1_0().cmd_copy_image_to_buffer(
                self.vk_cmd_buffer(),
                my_src.vk_image(),
                my_src.translate_layout(base::ImageLayout::CopyRead),
                my_dst.vk_buffer(),
                1,
                &vk::BufferImageCopy {
                    buffer_offset: dst_range.offset,
                    buffer_row_length: dst_range.row_stride as u32,
                    buffer_image_height: dst_range.plane_stride as u32,
                    image_subresource: my_src.resolve_vk_subresource_layers(
                        src_range,
                        translate_image_aspect(src_aspect),
                    ),
                    image_offset: vk::Offset3D {
                        x: src_origin[0] as i32,
                        y: src_origin[1] as i32,
                        z: src_origin[2] as i32,
                    },
                    image_extent: vk::Extent3D {
                        width: size[0],
                        height: size[1],
                        depth: size[2],
                    },
                },
            );
        }
    }

    fn copy_image(
        &mut self,
        src: &base::ImageRef,
        src_range: &base::ImageLayerRange,
        src_origin: &[u32],
        dst: &base::ImageRef,
        dst_range: &base::ImageLayerRange,
        dst_origin: &[u32],
        size: &[u32],
    ) {
        let my_src: &Image = src.downcast_ref().expect("bad source image type");
        let my_dst: &Image = dst.downcast_ref().expect("bad destination image type");

        let src_origin: [u32; 3] = src_origin.into_with_pad(0);
        let dst_origin: [u32; 3] = dst_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        assert_eq!(src_range.layers.len(), dst_range.layers.len());

        let src_aspect = my_src.aspects();
        let dst_aspect = my_dst.aspects();

        assert_eq!(
            src_aspect, dst_aspect,
            "source and destination format must match"
        );

        let vk_device = self.device.vk_device();

        let is_depth_stencil = src_aspect != vk::IMAGE_ASPECT_COLOR_BIT;

        unsafe {
            vk_device.cmd_copy_image(
                self.vk_cmd_buffer(),
                my_src.vk_image(),
                my_src.translate_layout(base::ImageLayout::CopyRead),
                my_dst.vk_image(),
                my_dst.translate_layout(base::ImageLayout::CopyWrite),
                &[vk::ImageCopy {
                    src_subresource: my_src.resolve_vk_subresource_layers(src_range, src_aspect),
                    src_offset: vk::Offset3D {
                        x: src_origin[0] as i32,
                        y: src_origin[1] as i32,
                        z: src_origin[2] as i32,
                    },
                    dst_subresource: my_dst.resolve_vk_subresource_layers(dst_range, dst_aspect),
                    dst_offset: vk::Offset3D {
                        x: dst_origin[0] as i32,
                        y: dst_origin[1] as i32,
                        z: dst_origin[2] as i32,
                    },
                    extent: vk::Extent3D {
                        width: size[0],
                        height: size[1],
                        depth: size[2],
                    },
                }],
            );
        }
    }
}
