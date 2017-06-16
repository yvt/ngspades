//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, metal, block};
use metal::NSObjectProtocol;
use enumflags::BitFlags;
use cgmath::Vector3;

use std::time::Duration;
use std::cell::RefCell;
use std::mem::{replace, drop, forget};
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};

use {ref_hash, OCPtr};
use imp::{Backend, Buffer, BufferView, ComputePipeline, DescriptorPool, DescriptorSet,
          DescriptorSetLayout, Fence, Framebuffer, GraphicsPipeline, Heap, Image, ImageView,
          PipelineLayout, RenderPass, Sampler, Semaphore, ShaderModule, StencilState};

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct CommandQueue {
    obj: OCPtr<metal::MTLCommandQueue>,
}

unsafe impl Send for CommandQueue {}

impl CommandQueue {
    pub(crate) fn new(obj: metal::MTLCommandQueue) -> Self {
        Self { obj: OCPtr::new(obj).unwrap() }
    }
}

struct SubmissionTransaction<'a> {
    submissions: &'a[&'a core::SubmissionInfo<'a, Backend>],
    num_successful_transitions: usize,
    fence_associated: Option<&'a Fence>,
}

fn submit_commands(submissions: &[&core::SubmissionInfo<Backend>],
                   fence: Option<&Fence>)
                   -> core::Result<()> {
    let mut transaction = SubmissionTransaction {
        submissions: submissions,
        num_successful_transitions: 0,
        fence_associated: None,
    };

    // Check some preconditions beforehand
    // (this eases error handling)
    for submission in submissions.iter() {
        for buffer in submission.buffers.iter() {
            buffer.buffer.as_ref().expect("invalid command buffer state");
            if buffer.encoder != EncoderState::NotRecording {
                panic!("invalid command buffer state");
            }
            // now we are sure this buffer is in the
            // `Executable`, `Pending`, or `Completed`
        }
    }

    let num_buffers = submissions.iter()
        .map(|s| s.buffers.len())
        .sum();

    // Make a state transition from `Executable` to `Pending`
    'check_state: for submission in submissions.iter() {
        for buffer in submission.buffers.iter() {
            let ov = buffer.submitted.swap(true, Ordering::Acquire);
            if ov {
                // Some buffers were not in `Executable`;
                panic!("invalid command buffer state");
            }
            transaction.num_successful_transitions += 1;
        }
    }

    let mut completion_handler = None;

    // Prepare fence
    if let Some(fence) = fence {
        let result = fence.associate_pending_buffers(num_buffers);
        transaction.fence_associated = Some(fence);

        // The fence must be unsignalled
        assert!(result, "fence must be in the unsignalled state");

        let fence_ref: Fence = fence.clone();
        let block = block::ConcreteBlock::new(move |cb| {
            fence_ref.remove_pending_buffers(1);
        });
        completion_handler = Some(block.copy());
    }

    for submission in submissions.iter() {
        for buffer in submission.buffers.iter() {
            let metal_buffer = buffer.buffer.as_ref().unwrap();
            if let Some(ref completion_handler) = completion_handler {
                metal_buffer.add_completed_handler(&**completion_handler);
            }

            metal_buffer.commit();

            // TODO: semaphores
        }
    }

    // The operation was successful; now commit the transaction
    forget(transaction);

    Ok(())
}

impl<'a> Drop for SubmissionTransaction<'a> {
    fn drop(&mut self) {
        // Perform rollback
        'rb_transitions: for submission in self.submissions.iter() {
            for buffer in submission.buffers.iter() {
                if self.num_successful_transitions == 0 {
                    break 'rb_transitions;
                }
                self.num_successful_transitions -= 1;
                buffer.submitted.store(false, Ordering::Release);
            }
        }

        if let Some(fence) = self.fence_associated {
            fence.remove_pending_buffers(self.num_successful_transitions);
        }
    }
}

impl core::CommandQueue<Backend> for CommandQueue {
    fn make_command_buffer(&self) -> core::Result<CommandBuffer> {
        Ok(CommandBuffer::new(*self.obj))
    }

    fn wait_idle(&self) {
        unimplemented!()
    }

    fn submit_commands(&self,
                       submissions: &[&core::SubmissionInfo<Backend>],
                       fence: Option<&Fence>)
                       -> core::Result<()> {
        submit_commands(submissions, fence)
    }
}

#[derive(Debug)]
pub struct CommandBuffer {
    queue: OCPtr<metal::MTLCommandQueue>,
    buffer: Option<OCPtr<metal::MTLCommandBuffer>>,
    encoder: EncoderState,
    submitted: AtomicBool,
}

#[derive(Debug, PartialEq, Eq)]
enum EncoderState {
    /// Completed recording.
    NotRecording,
    /// Recording has started, but not sure which encoder we should use.
    Undefined,
    Graphics{
        encoder: OCPtr<metal::MTLRenderCommandEncoder>,
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
        }
    }

    /// Completes the current encoder.
    /// Can't be used to end a graphics encoder. (note that
    /// graphics encoder is always terminated by `next_subpass` and `end_render_pass`)
    fn end_all_encoders(&mut self) {
        match self.encoder {
            EncoderState::NotRecording => unreachable!(),
            EncoderState::Undefined => {},
            EncoderState::Graphics{ .. } |
            EncoderState::GraphicsLast => {
                panic!("invalid opration during a render pass");
            },
            EncoderState::Compute(ref encoder) => {
                encoder.end_encoding();
            },
            EncoderState::Blit(ref encoder) => {
                encoder.end_encoding();
            },
        }
        self.encoder = EncoderState::Undefined;
    }

    fn expect_graphics_pipeline(&self) -> &OCPtr<metal::MTLRenderCommandEncoder> {
        if let EncoderState::Graphics { ref encoder, .. } = self.encoder {
            encoder
        } else {
            panic!("render subpass is not active");
        }
    }
}

impl core::CommandBuffer<Backend> for CommandBuffer {
    fn state(&self) -> core::CommandBufferState {
        match self.buffer {
            Some(ref buffer) =>
                match buffer.status() {
                    metal::MTLCommandBufferStatus::NotEnqueued =>
                        if let EncoderState::NotRecording = self.encoder {
                            core::CommandBufferState::Executable
                        } else {
                            core::CommandBufferState::Recording
                        },
                    metal::MTLCommandBufferStatus::Enqueued |
                    metal::MTLCommandBufferStatus::Committed |
                    metal::MTLCommandBufferStatus::Scheduled => core::CommandBufferState::Pending,
                    metal::MTLCommandBufferStatus::Completed => core::CommandBufferState::Completed,
                    metal::MTLCommandBufferStatus::Error => core::CommandBufferState::Error,
                },
            None => core::CommandBufferState::Initial
        }
    }
    fn wait_completion(&self, timeout: Duration) -> core::Result<bool> {
        // TODO: timeout
        self.buffer.as_ref().unwrap().wait_until_completed();
        Ok(true)
    }
}

impl core::CommandEncoder<Backend> for CommandBuffer {
    fn begin_encoding(&mut self) {
        let raw_buffer = self.queue.new_command_buffer();
        self.buffer = Some(OCPtr::new(raw_buffer).unwrap());
        self.encoder = EncoderState::Undefined;
    }

    fn end_encoding(&mut self) {
        self.end_all_encoders();
        self.encoder = EncoderState::NotRecording;
    }

    fn begin_render_pass(&mut self, framebuffer: &Framebuffer) {
        self.end_all_encoders();
        let first_subpass = framebuffer.subpass(0);
        self.encoder = EncoderState::Graphics{
            encoder: OCPtr::new(self.buffer.as_ref().unwrap().new_render_command_encoder(first_subpass)).unwrap(),
            framebuffer: framebuffer.clone(),
            subpass: 0,
        };
    }

    fn end_render_pass(&mut self) {
        match self.encoder {
            EncoderState::GraphicsLast => {
                self.encoder = EncoderState::Undefined;
            },
            EncoderState::Graphics {..} => {
                panic!("insufficient number of calls of next_pass");
            },
            _ => {
                panic!("render pass is not active");
            },
        }
    }

    fn next_subpass(&mut self) {
        match replace(&mut self.encoder, EncoderState::Undefined) {
            EncoderState::Graphics { encoder, framebuffer, subpass } => {
                encoder.end_encoding();
                drop(encoder);

                let next_subpass_index = subpass + 1;
                if next_subpass_index != framebuffer.num_subpasses() {
                    let next_subpass = framebuffer.subpass(next_subpass_index);
                    self.encoder = EncoderState::Graphics{
                        encoder: OCPtr::new(self.buffer.as_ref().unwrap().new_render_command_encoder(next_subpass)).unwrap(),
                        framebuffer: framebuffer,
                        subpass: next_subpass_index,
                    };
                } else {
                    self.encoder = EncoderState::GraphicsLast;
                }
            },
            EncoderState::GraphicsLast => {
                self.encoder = EncoderState::GraphicsLast;
                panic!("no more subpasses");
            },
            x => {
                self.encoder = x;
                panic!("render pass is not active");
            },
        }
    }
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline) {
        let gp = self.expect_graphics_pipeline();
        gp.set_render_pipeline_state(unimplemented!());

        // TODO: don't forget to set static states!
    }
    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        let gp = self.expect_graphics_pipeline();
        gp.set_blend_color(value[0], value[1], value[2], value[3]);
    }
    fn set_depth_bias(&mut self, value: Option<core::DepthBias>) {
        let gp = self.expect_graphics_pipeline();
        if let Some(value) = value {
            gp.set_depth_bias(value.constant_factor, value.slope_factor, value.clamp);
        } else {
            gp.set_depth_bias(0f32, 0f32, 0f32);
        }
    }
    fn set_depth_bounds(&mut self, _: Option<core::DepthBounds>) {
        panic!("not supported");
    }
    fn set_stencil_state(&mut self, value: &StencilState) {
        unimplemented!()
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        unimplemented!()
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        unimplemented!()
    }
    fn bind_descriptor_sets(&mut self,
                            pipeline_layout: &PipelineLayout,
                            start_index: usize,
                            descriptor_sets: &[DescriptorSet],
                            dynamic_offsets: &[u32]) {
        unimplemented!()
    }
    fn draw(&mut self,
            num_vertices: u32,
            num_instances: u32,
            start_vertex_index: u32,
            start_instance_index: u32) {
        unimplemented!()
    }
    fn draw_indexed(&mut self,
                    num_vertices: u32,
                    num_instances: u32,
                    start_vertex_index: u32,
                    index_offset: u32,
                    start_instance_index: u32) {
        unimplemented!()
    }

    fn bind_compute_pipeline(&mut self, pipeline: &ComputePipeline) {
        unimplemented!()
    }
    fn dispatch(&mut self, workgroup_count: Vector3<u32>) {
        unimplemented!()
    }
}
