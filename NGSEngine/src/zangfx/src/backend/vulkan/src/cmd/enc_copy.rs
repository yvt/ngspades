//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use std::ops::Range;

use base;
use device::DeviceRef;

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
    fn fill_buffer(&mut self, _buffer: &base::Buffer, _range: Range<base::DeviceSize>, _value: u8) {
        unimplemented!();
    }

    fn copy_buffer(
        &mut self,
        _src: &base::Buffer,
        _src_offset: base::DeviceSize,
        _dst: &base::Buffer,
        _dst_offset: base::DeviceSize,
        _size: base::DeviceSize,
    ) {
        unimplemented!();
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
