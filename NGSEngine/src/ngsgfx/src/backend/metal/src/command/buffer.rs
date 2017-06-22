//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, metal, OCPtr};
use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use std::mem::replace;
use enumflags::BitFlags;

use imp::{Backend, Framebuffer, SecondaryCommandBuffer};

use super::graphics::{GraphicsEncoderState, RenderCommandEncoder};

#[derive(Debug)]
pub struct CommandBuffer {
    queue: OCPtr<metal::MTLCommandQueue>,
    pub(crate) buffer: Option<OCPtr<metal::MTLCommandBuffer>>,
    pub(crate) encoder: EncoderState,
    pub(crate) submitted: AtomicBool,
    label: RefCell<Option<String>>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EncoderState {
    /// Completed recording.
    NotRecording,
    /// Recording has started, but not sure which encoder we should use.
    NoPass,
    Graphics {
        encoder: GraphicsEncoderState,
        framebuffer: Framebuffer,
        subpass: usize,
    },
    GraphicsLast,
    Compute(OCPtr<metal::MTLComputeCommandEncoder>),
    Blit(OCPtr<metal::MTLBlitCommandEncoder>),
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

    pub(crate) fn expect_no_pass(&self) {
        match self.encoder {
            EncoderState::NoPass => {}
            _ => {
                panic!("a pass is still active");
            }
        }
    }

    pub(crate) fn expect_graphics_pipeline(&self) -> &RenderCommandEncoder {
        if let EncoderState::Graphics {
            encoder: GraphicsEncoderState::Inline(ref encoder), ..
        } = self.encoder
        {
            encoder
        } else {
            panic!("inline render subpass is not active");
        }
    }

    pub(crate) fn expect_compute_pipeline(&self) -> &OCPtr<metal::MTLComputeCommandEncoder> {
        if let EncoderState::Compute(ref encoder) = self.encoder {
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
    fn wait_completion(&self, _: Duration) -> core::Result<bool> {
        // TODO: timeout
        self.buffer.as_ref().unwrap().wait_until_completed();
        Ok(true)
    }
}


impl core::CommandEncoder<Backend> for CommandBuffer {
    fn begin_encoding(&mut self) {
        let raw_buffer = self.queue.new_command_buffer();
        self.buffer = Some(OCPtr::new(raw_buffer).unwrap());
        self.encoder = EncoderState::NoPass;

        self.update_label();
    }

    fn end_encoding(&mut self) {
        self.expect_no_pass();
        self.encoder = EncoderState::NotRecording;
    }

    fn barrier(
        &mut self,
        source_stage: BitFlags<core::PipelineStageFlags>,
        destination_stage: BitFlags<core::PipelineStageFlags>,
        barriers: &[core::Barrier<Backend>],
    ) {
        unimplemented!()
    }

    fn begin_render_pass(&mut self, framebuffer: &Framebuffer, contents: core::RenderPassContents) {
        self.expect_no_pass();

        let first_subpass = framebuffer.subpass(0);
        let g_encoder = match contents {
            core::RenderPassContents::Inline => GraphicsEncoderState::Inline(
                RenderCommandEncoder::new(
                    OCPtr::new(
                        self.buffer
                            .as_ref()
                            .unwrap()
                            .new_render_command_encoder(first_subpass),
                    ).unwrap(),
                ),
            ),
            core::RenderPassContents::SecondaryCommandBuffers => {
                GraphicsEncoderState::SecondaryCommandBuffers(
                    OCPtr::new(
                        self.buffer
                            .as_ref()
                            .unwrap()
                            .new_parallel_render_command_encoder(first_subpass),
                    ).unwrap(),
                )
            }
        };
        self.encoder = EncoderState::Graphics {
            encoder: g_encoder,
            framebuffer: framebuffer.clone(),
            subpass: 0,
        };
    }

    fn begin_compute_pass(&mut self) {
        unimplemented!()
    }

    fn begin_blit_pass(&mut self) {
        unimplemented!()
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
            EncoderState::GraphicsLast => {
                self.encoder = EncoderState::NoPass;
            }
            EncoderState::Graphics { .. } => {
                panic!("insufficient number of calls of next_subpass");
            }
            EncoderState::Compute(ref encoder) => {
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
    }

    fn next_render_subpass(&mut self, contents: core::RenderPassContents) {
        match replace(&mut self.encoder, EncoderState::NoPass) {
            EncoderState::Graphics {
                encoder,
                framebuffer,
                subpass,
            } => {
                match encoder {
                    GraphicsEncoderState::Inline(ref encoder) => {
                        encoder.end_encoding()
                    }
                    GraphicsEncoderState::SecondaryCommandBuffers(ref encoder) => {
                        encoder.end_encoding()
                    }
                }
                drop(encoder);

                let next_subpass_index = subpass + 1;
                if next_subpass_index != framebuffer.num_subpasses() {
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
                } else {
                    self.encoder = EncoderState::GraphicsLast;
                }
            }
            EncoderState::GraphicsLast => {
                self.encoder = EncoderState::GraphicsLast;
                panic!("no more subpasses");
            }
            x => {
                self.encoder = x;
                panic!("render pass is not active");
            }
        }
    }
}
