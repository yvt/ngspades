//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use backend_vulkan;
use core;
use wsi_core;

use cgmath::Vector3;

#[derive(Debug)]
pub struct Drawable;

impl wsi_core::Drawable for Drawable {
    type Backend = backend_vulkan::ManagedBackend;

    fn image(&self) -> &<Self::Backend as core::Backend>::Image {
        unimplemented!()
    }
    fn acquiring_fence(&self) -> Option<&<Self::Backend as core::Backend>::Fence> {
        unimplemented!()
    }
    fn finalize(
        &self,
        command_buffer: &mut <Self::Backend as core::Backend>::CommandBuffer,
        state: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        layout: core::ImageLayout,
    ) {
        let _ = (command_buffer, state, access, layout);
        unimplemented!()
    }
    fn present(&self) {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct Swapchain;

impl wsi_core::Swapchain for Swapchain {
    type Backend = backend_vulkan::ManagedBackend;
    type Drawable = Drawable;

    fn device(&self) -> &<Self::Backend as core::Backend>::Device {
        unimplemented!()
    }
    fn next_drawable(
        &self,
        description: &wsi_core::FrameDescription,
    ) -> Result<Self::Drawable, wsi_core::SwapchainError> {
        let _ = description;
        unimplemented!()
    }
    fn image_extents(&self) -> Vector3<u32> {
        unimplemented!()
    }
    fn image_num_array_layers(&self) -> u32 {
        unimplemented!()
    }
    fn image_format(&self) -> core::ImageFormat {
        unimplemented!()
    }
    fn image_colorspace(&self) -> wsi_core::ColorSpace {
        unimplemented!()
    }
}
