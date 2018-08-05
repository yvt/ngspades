//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cocoa::foundation::NSRange;
use std::ops::Range;
use zangfx_metal_rs::{self as metal, MTLBlitCommandEncoder};

use zangfx_base::{self as base, DeviceSize};
use zangfx_base::{interfaces, vtable_for, zangfx_impl_object};
use zangfx_common::*;

use crate::buffer::Buffer;
use crate::cmd::enc::{CmdBufferFenceSet, DebugCommands};
use crate::cmd::fence::Fence;
use crate::image::Image;
use crate::utils::OCPtr;

#[derive(Debug)]
crate struct CopyEncoder {
    metal_encoder: OCPtr<MTLBlitCommandEncoder>,
    fence_set: CmdBufferFenceSet,
}

zangfx_impl_object! { CopyEncoder:
dyn base::CmdEncoder, dyn base::CopyCmdEncoder, dyn crate::Debug }

unsafe impl Send for CopyEncoder {}
unsafe impl Sync for CopyEncoder {}

impl CopyEncoder {
    crate unsafe fn new(
        metal_encoder: MTLBlitCommandEncoder,
        fence_set: CmdBufferFenceSet,
    ) -> Self {
        Self {
            metal_encoder: OCPtr::new(metal_encoder).unwrap(),
            fence_set,
        }
    }

    pub(super) fn finish(self) -> CmdBufferFenceSet {
        self.metal_encoder.end_encoding();
        self.fence_set
    }
}

impl base::CmdEncoder for CopyEncoder {
    fn begin_debug_group(&mut self, label: &str) {
        self.metal_encoder.begin_debug_group(label);
    }

    fn end_debug_group(&mut self) {
        self.metal_encoder.end_debug_group();
    }

    fn debug_marker(&mut self, label: &str) {
        self.metal_encoder.debug_marker(label);
    }

    fn use_resource_core(
        &mut self,
        _usage: base::ResourceUsageFlags,
        _objs: base::ResourceSet<'_>,
    ) {
        // No-op: no arguemnt table for copy encoder
    }

    fn use_heap(&mut self, _heaps: &[&base::HeapRef]) {
        // No-op: no arguemnt table for copy encoder
    }

    fn wait_fence(&mut self, fence: &base::FenceRef, _dst_access: base::AccessTypeFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.metal_encoder.wait_for_fence(our_fence.metal_fence());
        self.fence_set.wait_fence(our_fence);
    }

    fn update_fence(&mut self, fence: &base::FenceRef, _src_access: base::AccessTypeFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.metal_encoder.update_fence(our_fence.metal_fence());
        self.fence_set.signal_fence(our_fence);
    }

    fn barrier_core(
        &mut self,
        _obj: base::ResourceSet<'_>,
        _src_access: base::AccessTypeFlags,
        _dst_access: base::AccessTypeFlags,
    ) {
        // No-op: Metal's blit command encoders implicitly barrier between
        // each dispatch.
    }
}

impl base::CopyCmdEncoder for CopyEncoder {
    fn fill_buffer(&mut self, buffer: &base::BufferRef, range: Range<DeviceSize>, value: u8) {
        if range.start >= range.end {
            return;
        }
        let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        let (metal_buffer, buffer_offset) = my_buffer.metal_buffer_and_offset().unwrap();
        self.metal_encoder.fill_buffer(
            metal_buffer,
            NSRange::new(range.start + buffer_offset, range.end - range.start),
            value,
        );
    }

    fn copy_buffer(
        &mut self,
        src: &base::BufferRef,
        src_offset: DeviceSize,
        dst: &base::BufferRef,
        dst_offset: DeviceSize,
        size: DeviceSize,
    ) {
        let my_src: &Buffer = src.downcast_ref().expect("bad source buffer type");
        let my_dst: &Buffer = dst.downcast_ref().expect("bad destination buffer type");

        let (src_metal_buffer, src_buffer_offset) = my_src.metal_buffer_and_offset().unwrap();
        let (dst_metal_buffer, dst_buffer_offset) = my_dst.metal_buffer_and_offset().unwrap();

        self.metal_encoder.copy_from_buffer_to_buffer(
            src_metal_buffer,
            src_offset + src_buffer_offset,
            dst_metal_buffer,
            dst_offset + dst_buffer_offset,
            size,
        );
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
        let my_src: &Buffer = src.downcast_ref().expect("bad source buffer type");
        let my_dst: &Image = dst.downcast_ref().expect("bad destination image type");

        let dst_origin: [u32; 3] = dst_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        let (metal_buffer, buffer_offset) = my_src.metal_buffer_and_offset().unwrap();

        // TODO: `num_bytes_per_pixel` is incorrect for non-color images
        let pixel_size = my_dst.num_bytes_per_pixel();
        for i in dst_range.layers.clone() {
            self.metal_encoder.copy_from_buffer_to_image(
                metal_buffer,
                src_range.offset + buffer_offset
                    + src_range.plane_stride
                        * pixel_size as u64
                        * (i - dst_range.layers.start) as u64,
                src_range.row_stride * pixel_size as u64,
                src_range.plane_stride * pixel_size as u64,
                metal::MTLSize {
                    width: size[0] as u64,
                    height: size[1] as u64,
                    depth: size[2] as u64,
                },
                my_dst.metal_texture(),
                i as u64,
                dst_range.mip_level as u64,
                metal::MTLOrigin {
                    x: dst_origin[0] as u64,
                    y: dst_origin[1] as u64,
                    z: dst_origin[2] as u64,
                },
                match dst_aspect {
                    base::ImageAspect::Color => metal::MTLBlitOptionNone,
                    base::ImageAspect::Depth => metal::MTLBlitOptionDepthFromDepthStencil,
                    base::ImageAspect::Stencil => metal::MTLBlitOptionStencilFromDepthStencil,
                },
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

        let src_origin: [u32; 3] = src_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        let (metal_buffer, buffer_offset) = my_dst.metal_buffer_and_offset().unwrap();

        // TODO: `num_bytes_per_pixel` is incorrect for non-color images
        let pixel_size = my_src.num_bytes_per_pixel();
        for i in src_range.layers.clone() {
            self.metal_encoder.copy_from_image_to_buffer(
                my_src.metal_texture(),
                i as u64,
                src_range.mip_level as u64,
                metal::MTLOrigin {
                    x: src_origin[0] as u64,
                    y: src_origin[1] as u64,
                    z: src_origin[2] as u64,
                },
                metal::MTLSize {
                    width: size[0] as u64,
                    height: size[1] as u64,
                    depth: size[2] as u64,
                },
                metal_buffer,
                dst_range.offset + buffer_offset
                    + dst_range.plane_stride
                        * pixel_size as u64
                        * (i - src_range.layers.start) as u64,
                dst_range.row_stride * pixel_size as u64,
                dst_range.plane_stride * pixel_size as u64,
                match src_aspect {
                    base::ImageAspect::Color => metal::MTLBlitOptionNone,
                    base::ImageAspect::Depth => metal::MTLBlitOptionDepthFromDepthStencil,
                    base::ImageAspect::Stencil => metal::MTLBlitOptionStencilFromDepthStencil,
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

        for (src_layer, dst_layer) in src_range.layers.clone().zip(dst_range.layers.clone()) {
            self.metal_encoder.copy_from_image_to_image(
                my_src.metal_texture(),
                src_layer as u64,
                src_range.mip_level as u64,
                metal::MTLOrigin {
                    x: src_origin[0] as u64,
                    y: src_origin[1] as u64,
                    z: src_origin[2] as u64,
                },
                metal::MTLSize {
                    width: size[0] as u64,
                    height: size[1] as u64,
                    depth: size[2] as u64,
                },
                my_dst.metal_texture(),
                dst_layer as u64,
                dst_range.mip_level as u64,
                metal::MTLOrigin {
                    x: dst_origin[0] as u64,
                    y: dst_origin[1] as u64,
                    z: dst_origin[2] as u64,
                },
            );
        }
    }
}
