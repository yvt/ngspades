//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, metal};

use super::graphics::RenderCommandEncoder;

use imp::Backend;

#[derive(Debug)]
pub struct SecondaryCommandBuffer {
    encoder: RenderCommandEncoder,
}

impl SecondaryCommandBuffer {
    pub(crate) fn new(encoder: RenderCommandEncoder) -> Self {
        Self { encoder }
    }

    pub(crate) fn render_command_encoder(&mut self) -> &mut RenderCommandEncoder {
        &mut self.encoder
    }

    pub(crate) fn metal_command_encoder(&mut self) -> metal::MTLCommandEncoder {
        self.encoder.metal_command_encoder()
    }
}

unsafe impl Send for SecondaryCommandBuffer {}

impl core::SecondaryCommandBuffer<Backend> for SecondaryCommandBuffer {
    fn end_encoding(&mut self) {
        self.encoder.end_encoding();
    }
}

impl core::Marker for SecondaryCommandBuffer {
    fn set_label(&self, label: Option<&str>) {
        self.encoder.set_label(label.unwrap_or(""));
    }
}
