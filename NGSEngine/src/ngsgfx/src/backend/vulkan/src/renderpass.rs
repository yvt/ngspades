//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use {RefEqArc, DeviceRef, Backend};

pub struct RenderPass<T: DeviceRef> {
    data: RefEqArc<RenderPassData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for RenderPass<T> => data
}

#[derive(Debug)]
struct RenderPassData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::RenderPass for RenderPass<T> {}

impl<T: DeviceRef> core::Marker for RenderPass<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
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
    device: T,
}

impl<T: DeviceRef> core::Framebuffer for Framebuffer<T> {}

impl<T: DeviceRef> core::Marker for Framebuffer<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
