//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, metal, OCPtr};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::mem::replace;

use imp::{Backend, Framebuffer, SecondaryCommandBuffer};

use super::graphics::{GraphicsEncoderState, RenderCommandEncoder};
use super::compute::ComputeCommandEncoder;

#[derive(Debug)]
pub struct CommandBuffer {
    queue: OCPtr<metal::MTLCommandQueue>,
    pub(crate) buffer: Option<OCPtr<metal::MTLCommandBuffer>>,
    pub(crate) encoder: EncoderState,
    pub(crate) submitted: AtomicBool,
    label: RefCell<Option<String>>,
}

#[derive(Debug)]
pub(crate) enum EncoderState {
    /// Completed recording.
    NotRecording,
    /// Recording has started, but not sure which encoder we should use.
    NoPass,
    GraphicsIntermission {
        framebuffer: Framebuffer,
        next_subpass: usize,
    },
    Graphics {
        encoder: GraphicsEncoderState,
        framebuffer: Framebuffer,
        subpass: usize,
    },
    Compute(ComputeCommandEncoder),
    Blit(OCPtr<metal::MTLBlitCommandEncoder>),
}

impl EncoderState {
    pub fn is_recording(&self) -> bool {
        match self {
            &EncoderState::NotRecording => false,
            _ => true,
        }
    }
}

unsafe impl Send for CommandBuffer {}

impl CommandBuffer {
    pub(crate) fn new(queue: metal::MTLCommandQueue) -> Self {
        Self {
            queue: OCPtr::new(queue).unwrap(),
            buffer: None,
            encoder: EncoderState::NotRecording,
            submitted: AtomicBool::new(false),
            label: RefCell::new(None),
        }
    }

    pub fn metal_command_buffer(&self) -> Option<metal::MTLCommandBuffer> {
        self.buffer.as_ref().map(|x| **x)
    }

    pub(crate) fn expect_no_pass(&self) {
        match self.encoder {
            EncoderState::NoPass => {}
            _ => {
                panic!("a pass is still active");
            }
        }
    }

    /// Be careful about the returned object; it is guaranteed to be valid only
    /// as long as `self` (functions of `metal-rs` are not marked as `unsafe`
    /// in spite of that!)
    pub(crate) fn expect_command_encoder(&self) -> metal::MTLCommandEncoder {
        match self.encoder {
            EncoderState::Graphics {
                encoder: GraphicsEncoderState::SecondaryCommandBuffers(..), ..
            } => {
                panic!("cannot encode a debug command into SCB render subpass");
            }
            EncoderState::Graphics {
                encoder: GraphicsEncoderState::Inline(ref encoder), ..
            } => encoder.metal_command_encoder(),
            EncoderState::Compute(ref encoder) => encoder.metal_command_encoder(),
            EncoderState::Blit(ref encoder) => ***encoder,
            EncoderState::NoPass |
            EncoderState::NotRecording => {
                panic!("pass is not active");
            }
            EncoderState::GraphicsIntermission { .. } => {
                panic!("render subpass is not active");
            }
        }
    }

    pub(crate) fn expect_graphics_pipeline(&mut self) -> &mut RenderCommandEncoder {
        if let EncoderState::Graphics {
            encoder: GraphicsEncoderState::Inline(ref mut encoder), ..
        } = self.encoder
        {
            encoder
        } else {
            panic!("inline render subpass is not active");
        }
    }

    pub(crate) fn expect_compute_pipeline(&mut self) -> &mut ComputeCommandEncoder {
        if let EncoderState::Compute(ref mut encoder) = self.encoder {
            encoder
        } else {
            panic!("compute pass is not active");
        }
    }

    pub(crate) fn expect_blit_pipeline(&self) -> &OCPtr<metal::MTLBlitCommandEncoder> {
        if let EncoderState::Blit(ref encoder) = self.encoder {
            encoder
        } else {
            panic!("blit pass is not active");
        }
    }

    pub(crate) fn update_label(&self) {
        if let (Some(buffer), Some(label)) = (self.buffer.as_ref(), self.label.borrow().as_ref()) {
            buffer.set_label(label);
        }
    }
}

impl core::Marker for CommandBuffer {
    fn set_label(&self, label: Option<&str>) {
        *self.label.borrow_mut() = Some(label.map(String::from).unwrap_or_else(String::new));

        self.update_label();
    }
}

impl core::CommandBuffer<Backend> for CommandBuffer {
    fn state(&self) -> core::CommandBufferState {
        match self.buffer {
            Some(ref buffer) => {
                match buffer.status() {
                    metal::MTLCommandBufferStatus::NotEnqueued => {
                        if let EncoderState::NotRecording = self.encoder {
                            core::CommandBufferState::Executable
                        } else {
                            core::CommandBufferState::Recording
                        }
                    }
                    metal::MTLCommandBufferStatus::Enqueued |
                    metal::MTLCommandBufferStatus::Committed |
                    metal::MTLCommandBufferStatus::Scheduled => core::CommandBufferState::Pending,
                    metal::MTLCommandBufferStatus::Completed => core::CommandBufferState::Completed,
                    metal::MTLCommandBufferStatus::Error => core::CommandBufferState::Error,
                }
            }
            None => core::CommandBufferState::Initial,
        }
    }
    fn wait_completion(&self) -> core::Result<()> {
        if let Some(ref buffer) = self.buffer {
            buffer.wait_until_completed();
        }
        Ok(())
    }
}


impl core::CommandEncoder<Backend> for CommandBuffer {
    fn begin_encoding(&mut self) {
        // FIXME: check current state?

        let raw_buffer = self.queue.new_command_buffer();
        self.buffer = Some(OCPtr::new(raw_buffer).unwrap());
        self.encoder = EncoderState::NoPass;
        self.submitted.store(false, Ordering::Relaxed); // TODO: care about ordering

        self.update_label();
    }

    fn end_encoding(&mut self) {
        self.expect_no_pass();
        self.encoder = EncoderState::NotRecording;
    }

    fn begin_render_pass(&mut self, framebuffer: &Framebuffer, _: core::DeviceEngine) {
        self.expect_no_pass();
        self.encoder = EncoderState::GraphicsIntermission {
            framebuffer: framebuffer.clone(),
            next_subpass: 0,
        };
    }

    fn begin_compute_pass(&mut self, _: core::DeviceEngine) {
        self.expect_no_pass();

        let encoder = OCPtr::new(self.buffer.as_ref().unwrap().new_compute_command_encoder());
        self.encoder = EncoderState::Compute(ComputeCommandEncoder::new(encoder.unwrap()));
    }

    fn begin_blit_pass(&mut self, _: core::DeviceEngine) {
        self.expect_no_pass();

        let encoder = OCPtr::new(self.buffer.as_ref().unwrap().new_blit_command_encoder());
        self.encoder = EncoderState::Blit(encoder.unwrap());
    }

    fn make_secondary_command_buffer(&mut self) -> SecondaryCommandBuffer {
        if let EncoderState::Graphics {
            encoder: GraphicsEncoderState::SecondaryCommandBuffers(ref prce), ..
        } = self.encoder
        {
            OCPtr::new(prce.render_command_encoder())
                .map(RenderCommandEncoder::new)
                .map(SecondaryCommandBuffer::new)
                .unwrap()
        } else {
            panic!("secondary command buffer render pass is not active");
        }
    }

    fn end_pass(&mut self) {
        match self.encoder {
            EncoderState::GraphicsIntermission {
                next_subpass,
                ref framebuffer,
            } => {
                if next_subpass < framebuffer.num_subpasses() {
                    panic!("insufficient number of calls of next_subpass");
                }
            }
            EncoderState::Graphics { .. } => {
                panic!("render subpass must be ended first");
            }
            EncoderState::Compute(ref mut encoder) => {
                encoder.end_encoding();
            }
            EncoderState::Blit(ref encoder) => {
                encoder.end_encoding();
            }
            EncoderState::NoPass |
            EncoderState::NotRecording => {
                panic!("render pass is not active");
            }
        }
        self.encoder = EncoderState::NoPass;
    }

    fn end_render_subpass(&mut self) {
        match replace(&mut self.encoder, EncoderState::NoPass) {
            EncoderState::Graphics {
                mut encoder,
                framebuffer,
                subpass,
            } => {
                match encoder {
                    GraphicsEncoderState::Inline(ref mut encoder) => encoder.end_encoding(),
                    GraphicsEncoderState::SecondaryCommandBuffers(ref mut encoder) => {
                        encoder.end_encoding()
                    }
                }
                drop(encoder);

                self.encoder = EncoderState::GraphicsIntermission {
                    framebuffer: framebuffer,
                    next_subpass: subpass + 1,
                };
            }
            x => {
                self.encoder = x;
                panic!("render subpass is not active");
            }
        }
    }

    fn begin_render_subpass(&mut self, contents: core::RenderPassContents) {
        match replace(&mut self.encoder, EncoderState::NoPass) {
            EncoderState::GraphicsIntermission {
                framebuffer,
                next_subpass: next_subpass_index,
            } => {
                if next_subpass_index == framebuffer.num_subpasses() {
                    panic!("no more subpasses");
                }
                let next_subpass = framebuffer.subpass(next_subpass_index);
                let g_encoder = match contents {
                    core::RenderPassContents::Inline => GraphicsEncoderState::Inline(
                        RenderCommandEncoder::new(
                            OCPtr::new(
                                self.buffer.as_ref().unwrap().new_render_command_encoder(
                                    next_subpass,
                                ),
                            ).unwrap(),
                        ),
                    ),
                    core::RenderPassContents::SecondaryCommandBuffers => {
                        GraphicsEncoderState::SecondaryCommandBuffers(
                            OCPtr::new(
                                self.buffer
                                    .as_ref()
                                    .unwrap()
                                    .new_parallel_render_command_encoder(next_subpass),
                            ).unwrap(),
                        )
                    }
                };
                self.encoder = EncoderState::Graphics {
                    encoder: g_encoder,
                    framebuffer: framebuffer,
                    subpass: next_subpass_index,
                };
            }
            x => {
                self.encoder = x;
                panic!("render pass is not active");
            }
        }
    }
}
