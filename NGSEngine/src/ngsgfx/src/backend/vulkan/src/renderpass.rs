//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::version::DeviceV1_0;
use ash::vk;
use std::ptr;

use {RefEqArc, DeviceRef, Backend, AshDevice};

pub struct RenderPass<T: DeviceRef> {
    data: RefEqArc<RenderPassData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for RenderPass<T> => data
}

#[derive(Debug)]
struct RenderPassData<T: DeviceRef> {
    device_ref: T,
    handle: vk::RenderPass,
}

impl<T: DeviceRef> Drop for RenderPassData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.destroy_render_pass(self.handle, self.device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> core::RenderPass for RenderPass<T> {}

impl<T: DeviceRef> core::Marker for RenderPass<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> RenderPass<T> {
    pub fn handle(&self) -> vk::RenderPass {
        self.data.handle
    }
}

pub struct Framebuffer<T: DeviceRef> {
    data: RefEqArc<FramebufferData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Framebuffer<T> => data
}

#[derive(Debug)]
struct FramebufferData<T: DeviceRef> {
    device_ref: T,
    handle: vk::Framebuffer,
    clear_values: Vec<vk::ClearValue>,
    num_subpasses: usize,
    render_pass: RenderPass<T>,
    extent: vk::Extent2D,
}

impl<T: DeviceRef> Drop for FramebufferData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.destroy_framebuffer(self.handle, self.device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> core::Framebuffer for Framebuffer<T> {}

impl<T: DeviceRef> core::Marker for Framebuffer<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> Framebuffer<T> {
    pub(crate) fn num_subpasses(&self) -> usize {
        self.data.num_subpasses
    }

    pub(crate) fn render_pass_begin_info(&self) -> vk::RenderPassBeginInfo {
        vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RenderPassBeginInfo,
            p_next: ptr::null(),
            render_pass: self.data.render_pass.handle(),
            framebuffer: self.data.handle,
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.data.extent.clone(),
            },
            clear_value_count: self.data.clear_values.len() as u32,
            p_clear_values: self.data.clear_values.as_ptr(),
        }
    }

    pub fn handle(&self) -> vk::Framebuffer {
        self.data.handle
    }
}
