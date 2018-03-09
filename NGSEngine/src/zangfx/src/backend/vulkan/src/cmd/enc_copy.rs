//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use ash::version::*;
use std::ops::Range;

use base;
use device::DeviceRef;
use buffer::Buffer;

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

    fn use_resource(&mut self, _usage: base::ResourceUsage, _objs: &[base::ResourceRef]) {
        // No-op on Vulkan backend
    }

    fn use_heap(&mut self, _heaps: &[&base::Heap]) {
        // No-op on Vulkan backend
    }

    fn wait_fence(
        &mut self,
        fence: &base::Fence,
        src_stage: base::StageFlags,
        barrier: &base::Barrier,
    ) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.common().wait_fence(&our_fence, src_stage, barrier);
        self.fence_set.wait_fence(our_fence);
    }

    fn update_fence(&mut self, fence: &base::Fence, src_stage: base::StageFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.common().update_fence(&our_fence, src_stage);
        self.fence_set.signal_fence(our_fence);
    }

    fn barrier(&mut self, barrier: &base::Barrier) {
        self.common().barrier(barrier)
    }
}

impl base::CopyCmdEncoder for CopyEncoder {
    fn fill_buffer(&mut self, buffer: &base::Buffer, range: Range<base::DeviceSize>, value: u8) {
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
        src: &base::Buffer,
        src_offset: base::DeviceSize,
        dst: &base::Buffer,
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
        _src: &base::Buffer,
        _src_range: &base::BufferImageRange,
        _dst: &base::Image,
        _dst_layout: base::ImageLayout,
        _dst_aspect: base::ImageAspect,
        _dst_range: &base::ImageLayerRange,
        _dst_origin: &[u32],
        _size: &[u32],
    ) {
        unimplemented!();
    }

    fn copy_image_to_buffer(
        &mut self,
        _src: &base::Image,
        _src_layout: base::ImageLayout,
        _src_aspect: base::ImageAspect,
        _src_range: &base::ImageLayerRange,
        _src_origin: &[u32],
        _dst: &base::Buffer,
        _dst_range: &base::BufferImageRange,
        _size: &[u32],
    ) {
        unimplemented!();
    }

    fn copy_image(
        &mut self,
        _src: &base::Image,
        _src_layout: base::ImageLayout,
        _src_range: &base::ImageLayerRange,
        _src_origin: &[u32],
        _dst: &base::Image,
        _dst_layout: base::ImageLayout,
        _dst_range: &base::ImageLayerRange,
        _dst_origin: &[u32],
        _size: &[u32],
    ) {
        unimplemented!();
    }
}
