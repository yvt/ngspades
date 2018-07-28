//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use ash::version::*;
use std::ops::Range;

use base;
use common::IntoWithPad;

use device::DeviceRef;
use buffer::Buffer;
use image::Image;
use utils::{translate_image_aspect, translate_image_layout, translate_image_subresource_layers};
use super::enc::{CommonCmdEncoder, FenceSet};
use super::fence::Fence;

#[derive(Debug)]
pub(super) struct CopyEncoder {
    device: DeviceRef,
    vk_cmd_buffer: vk::CommandBuffer,
    fence_set: FenceSet,
}

zangfx_impl_object! { CopyEncoder:
base::CmdEncoder, base::CopyCmdEncoder, ::Debug }

impl CopyEncoder {
    pub unsafe fn new(
        device: DeviceRef,
        vk_cmd_buffer: vk::CommandBuffer,
        fence_set: FenceSet,
    ) -> Self {
        Self {
            device,
            vk_cmd_buffer,
            fence_set,
        }
    }

    pub fn finish(self) -> FenceSet {
        self.fence_set
    }

    fn common(&self) -> CommonCmdEncoder {
        CommonCmdEncoder::new(self.device, self.vk_cmd_buffer)
    }
}

impl base::CmdEncoder for CopyEncoder {
    fn begin_debug_group(&mut self, label: &str) {
        self.common().begin_debug_group(label)
    }

    fn end_debug_group(&mut self) {
        self.common().end_debug_group()
    }

    fn debug_marker(&mut self, label: &str) {
        self.common().debug_marker(label)
    }

    fn use_resource_core(&mut self, _usage: base::ResourceUsageFlags, _objs: base::ResourceSet<'_>) {
        unimplemented!()
    }

    fn use_heap(&mut self, _heaps: &[&base::HeapRef]) {
        unimplemented!()
    }

    fn wait_fence(
        &mut self,
        fence: &base::FenceRef,
        dst_access: base::AccessTypeFlags,
    ) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.common().wait_fence(&our_fence, dst_access);
        self.fence_set.wait_fence(our_fence);
    }

    fn update_fence(&mut self, fence: &base::FenceRef, src_access: base::AccessTypeFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.common().update_fence(&our_fence, src_access);
        self.fence_set.signal_fence(our_fence);
    }

    fn barrier_core(
        &mut self,
        obj: base::ResourceSet<'_>,
        src_access: base::AccessTypeFlags,
        dst_access: base::AccessTypeFlags,
    ) {
        self.common().barrier_core(obj, src_access, dst_access)
    }
}

impl base::CopyCmdEncoder for CopyEncoder {
    fn fill_buffer(&mut self, buffer: &base::BufferRef, range: Range<base::DeviceSize>, value: u8) {
        if range.start >= range.end {
            return;
        }
        let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        let vk_device = self.device.vk_device();

        let data = (value as u32) * 0x1010101;

        unsafe {
            vk_device.cmd_fill_buffer(
                self.vk_cmd_buffer,
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

        unsafe {
            vk_device.cmd_copy_buffer(
                self.vk_cmd_buffer,
                my_src.vk_buffer(),
                my_dst.vk_buffer(),
                &[
                    vk::BufferCopy {
                        src_offset,
                        dst_offset,
                        size,
                    },
                ],
            );
        }
    }

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
        let dst_layout = unimplemented!();

        let my_src: &Buffer = src.downcast_ref().expect("bad source buffer type");
        let my_dst: &Image = dst.downcast_ref().expect("bad destination image type");

        let dst_origin: [u32; 3] = dst_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        let vk_device = self.device.vk_device();

        unsafe {
            vk_device.cmd_copy_buffer_to_image(
                self.vk_cmd_buffer,
                my_src.vk_buffer(),
                my_dst.vk_image(),
                translate_image_layout(dst_layout, dst_aspect != base::ImageAspect::Color),
                &[
                    vk::BufferImageCopy {
                        buffer_offset: src_range.offset,
                        buffer_row_length: src_range.row_stride as u32,
                        buffer_image_height: src_range.plane_stride as u32,
                        image_subresource: translate_image_subresource_layers(
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
                    },
                ],
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
        let src_layout = unimplemented!();

        let my_src: &Image = src.downcast_ref().expect("bad source image type");
        let my_dst: &Buffer = dst.downcast_ref().expect("bad destination buffer type");

        let src_origin: [u32; 3] = src_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        let vk_device = self.device.vk_device();

        unsafe {
            vk_device.fp_v1_0().cmd_copy_image_to_buffer(
                self.vk_cmd_buffer,
                my_src.vk_image(),
                translate_image_layout(src_layout, src_aspect != base::ImageAspect::Color),
                my_dst.vk_buffer(),
                1,
                &vk::BufferImageCopy {
                    buffer_offset: dst_range.offset,
                    buffer_row_length: dst_range.row_stride as u32,
                    buffer_image_height: dst_range.plane_stride as u32,
                    image_subresource: translate_image_subresource_layers(
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
        let src_layout = unimplemented!();
        let dst_layout = unimplemented!();

        let my_src: &Image = src.downcast_ref().expect("bad source image type");
        let my_dst: &Image = dst.downcast_ref().expect("bad destination image type");

        let src_origin: [u32; 3] = src_origin.into_with_pad(0);
        let dst_origin: [u32; 3] = dst_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        assert_eq!(src_range.layers.len(), dst_range.layers.len());

        let src_aspect = my_src.meta().image_aspects();
        let dst_aspect = my_dst.meta().image_aspects();

        assert_eq!(
            src_aspect, dst_aspect,
            "source and destination format must match"
        );

        let vk_device = self.device.vk_device();

        let is_depth_stencil = src_aspect != vk::IMAGE_ASPECT_COLOR_BIT;

        unsafe {
            vk_device.cmd_copy_image(
                self.vk_cmd_buffer,
                my_src.vk_image(),
                translate_image_layout(src_layout, is_depth_stencil),
                my_dst.vk_image(),
                translate_image_layout(dst_layout, is_depth_stencil),
                &[
                    vk::ImageCopy {
                        src_subresource: translate_image_subresource_layers(src_range, src_aspect),
                        src_offset: vk::Offset3D {
                            x: src_origin[0] as i32,
                            y: src_origin[1] as i32,
                            z: src_origin[2] as i32,
                        },
                        dst_subresource: translate_image_subresource_layers(dst_range, dst_aspect),
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
                    },
                ],
            );
        }
    }
}
