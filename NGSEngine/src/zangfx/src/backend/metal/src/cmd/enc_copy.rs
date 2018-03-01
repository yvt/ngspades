//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;
use metal::MTLBlitCommandEncoder;
use cocoa::foundation::NSRange;
use base::{command, handles, heap, resources, DeviceSize, StageFlags};

use utils::OCPtr;
use cmd::enc::CmdBufferFenceSet;
use cmd::fence::Fence;

use buffer::Buffer;

#[derive(Debug)]
pub struct CopyEncoder {
    metal_encoder: OCPtr<MTLBlitCommandEncoder>,
    fence_set: CmdBufferFenceSet,
}

zangfx_impl_object! { CopyEncoder:
command::CmdEncoder, command::CopyCmdEncoder, ::Debug }

impl CopyEncoder {
    pub unsafe fn new(metal_encoder: MTLBlitCommandEncoder, fence_set: CmdBufferFenceSet) -> Self {
        Self {
            metal_encoder: OCPtr::new(metal_encoder).unwrap(),
            fence_set,
        }
    }

    pub fn finish(self) -> CmdBufferFenceSet {
        self.fence_set
    }
}

impl command::CmdEncoder for CopyEncoder {
    fn use_resource(&mut self, _usage: command::ResourceUsage, _objs: &[handles::ResourceRef]) {
        // No-op: no arguemnt table for copy encoder
    }

    fn use_heap(&mut self, _heaps: &[&heap::Heap]) {
        // No-op: no arguemnt table for copy encoder
    }

    fn wait_fence(
        &mut self,
        fence: &handles::Fence,
        _src_stage: StageFlags,
        _dst_stage: StageFlags,
        _barrier: &handles::Barrier,
    ) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.metal_encoder.wait_for_fence(our_fence.metal_fence());
        self.fence_set.wait_fences.push(our_fence);
    }

    fn update_fence(&mut self, fence: &handles::Fence, _src_stage: StageFlags) {
        let our_fence = Fence::clone(fence.downcast_ref().expect("bad fence type"));
        self.metal_encoder.update_fence(our_fence.metal_fence());
        self.fence_set.signal_fences.push(our_fence);
    }

    fn barrier(
        &mut self,
        _src_stage: StageFlags,
        _dst_stage: StageFlags,
        _barrier: &handles::Barrier,
    ) {
        // No-op: Metal's blit command encoders implicitly barrier between
        // each dispatch.
    }
}

impl command::CopyCmdEncoder for CopyEncoder {
    fn fill_buffer(&mut self, buffer: &handles::Buffer, range: Range<DeviceSize>, value: u8) {
        if range.start >= range.end {
            return;
        }
        let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        self.metal_encoder.fill_buffer(
            my_buffer.metal_buffer(),
            NSRange::new(range.start, range.end - range.start),
            value,
        );
    }

    fn copy_buffer(
        &mut self,
        src: &handles::Buffer,
        src_offset: DeviceSize,
        dst: &handles::Buffer,
        dst_offset: DeviceSize,
        size: DeviceSize,
    ) {
        let my_src: &Buffer = src.downcast_ref().expect("bad source buffer type");
        let my_dst: &Buffer = dst.downcast_ref().expect("bad destination buffer type");

        self.metal_encoder.copy_from_buffer_to_buffer(
            my_src.metal_buffer(),
            src_offset as u64,
            my_dst.metal_buffer(),
            dst_offset as u64,
            size as u64,
        );
    }

    fn copy_buffer_to_image(
        &mut self,
        _src: &handles::Buffer,
        _src_range: &command::BufferImageRange,
        _dst: &handles::Image,
        _dst_layout: resources::ImageLayout,
        _dst_aspect: resources::ImageAspect,
        _dst_range: &resources::ImageLayerRange,
        _dst_origin: &[u32],
        _size: &[u32],
    ) {
        unimplemented!();
    }

    fn copy_image_to_buffer(
        &mut self,
        _src: &handles::Image,
        _src_layout: resources::ImageLayout,
        _src_aspect: resources::ImageAspect,
        _src_range: &resources::ImageLayerRange,
        _src_origin: &[u32],
        _dst: &handles::Buffer,
        _dst_range: &command::BufferImageRange,
        _size: &[u32],
    ) {
        unimplemented!();
    }

    fn copy_image(
        &mut self,
        _src: &handles::Image,
        _src_layout: resources::ImageLayout,
        _src_range: &resources::ImageLayerRange,
        _src_origin: &[u32],
        _dst: &handles::Image,
        _dst_layout: resources::ImageLayout,
        _dst_range: &resources::ImageLayerRange,
        _dst_origin: &[u32],
        _size: &[u32],
    ) {
        unimplemented!();
    }
}
