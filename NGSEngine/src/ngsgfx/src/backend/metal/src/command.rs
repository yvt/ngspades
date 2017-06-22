//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, metal, block};
use enumflags::BitFlags;
use cgmath::Vector3;

use std::time::Duration;
use std::cell::RefCell;
use std::mem::{replace, drop, forget};
use std::sync::atomic::{AtomicBool, Ordering};

use OCPtr;
use imp::{Backend, Buffer, ComputePipeline, DescriptorPool, DescriptorSet,
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
    submissions: &'a [&'a core::SubmissionInfo<'a, Backend>],
    num_successful_transitions: usize,
    fence_associated: Option<&'a Fence>,
}

fn submit_commands(
    submissions: &[&core::SubmissionInfo<Backend>],
    fence: Option<&Fence>,
) -> core::Result<()> {
    let mut transaction = SubmissionTransaction {
        submissions: submissions,
        num_successful_transitions: 0,
        fence_associated: None,
    };

    // Check some preconditions beforehand
    // (this eases error handling)
    for submission in submissions.iter() {
        for buffer in submission.buffers.iter() {
            buffer.buffer.as_ref().expect(
                "invalid command buffer state",
            );
            if buffer.encoder != EncoderState::NotRecording {
                panic!("invalid command buffer state");
            }
            // now we are sure this buffer is in the
            // `Executable`, `Pending`, or `Completed`
        }
    }

    let num_buffers = submissions.iter().map(|s| s.buffers.len()).sum();

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
        let block = block::ConcreteBlock::new(move |_| { fence_ref.remove_pending_buffers(1); });
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

impl core::Marker for CommandQueue {
    fn set_label(&self, label: Option<&str>) {
        self.obj.set_label(label.unwrap_or(""));
    }
}

impl core::CommandQueue<Backend> for CommandQueue {
    fn make_command_buffer(&self) -> core::Result<CommandBuffer> {
        Ok(CommandBuffer::new(*self.obj))
    }

    fn wait_idle(&self) {
        unimplemented!()
    }

    fn submit_commands(
        &self,
        submissions: &[&core::SubmissionInfo<Backend>],
        fence: Option<&Fence>,
    ) -> core::Result<()> {
        submit_commands(submissions, fence)
    }
}

#[derive(Debug)]
pub struct CommandBuffer {
    queue: OCPtr<metal::MTLCommandQueue>,
    buffer: Option<OCPtr<metal::MTLCommandBuffer>>,
    encoder: EncoderState,
    submitted: AtomicBool,
    label: RefCell<Option<String>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SecondaryCommandBuffer {
    encoder: RenderCommandEncoder,
}

#[derive(Debug, PartialEq, Eq)]
struct RenderCommandEncoder {
    metal_encoder: OCPtr<metal::MTLRenderCommandEncoder>,
}

#[derive(Debug, PartialEq, Eq)]
enum EncoderState {
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

#[derive(Debug, PartialEq, Eq)]
enum GraphicsEncoderState {
    Inline(RenderCommandEncoder),
    SecondaryCommandBuffers(OCPtr<metal::MTLParallelRenderCommandEncoder>),
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

    fn expect_no_pass(&self) {
        match self.encoder {
            EncoderState::NoPass => {}
            _ => {
                panic!("a pass is still active");
            }
        }
    }

    fn expect_graphics_pipeline(&self) -> &RenderCommandEncoder {
        if let EncoderState::Graphics {
            encoder: GraphicsEncoderState::Inline(ref encoder), ..
        } = self.encoder
        {
            encoder
        } else {
            panic!("inline render subpass is not active");
        }
    }

    fn expect_compute_pipeline(&self) -> &OCPtr<metal::MTLComputeCommandEncoder> {
        if let EncoderState::Compute(ref encoder) = self.encoder {
            encoder
        } else {
            panic!("compute pass is not active");
        }
    }

    fn expect_blit_pipeline(&self) -> &OCPtr<metal::MTLBlitCommandEncoder> {
        if let EncoderState::Blit(ref encoder) = self.encoder {
            encoder
        } else {
            panic!("blit pass is not active");
        }
    }

    fn update_label(&self) {
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
                        encoder.metal_encoder.end_encoding()
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

impl RenderCommandEncoder {
    fn new(metal_encoder: OCPtr<metal::MTLRenderCommandEncoder>) -> Self {
        Self { metal_encoder }
    }

    fn bind_graphics_pipeline(&self, pipeline: &GraphicsPipeline) {
        self.metal_encoder.set_render_pipeline_state(
            unimplemented!(),
        );

        // TODO: don't forget to set static states!
    }
    fn set_blend_constants(&self, value: &[f32; 4]) {
        self.metal_encoder.set_blend_color(
            value[0],
            value[1],
            value[2],
            value[3],
        );
    }
    fn set_depth_bias(&self, value: Option<core::DepthBias>) {
        if let Some(value) = value {
            self.metal_encoder.set_depth_bias(
                value.constant_factor,
                value.slope_factor,
                value.clamp,
            );
        } else {
            self.metal_encoder.set_depth_bias(0f32, 0f32, 0f32);
        }
    }
    fn set_depth_bounds(&self, _: Option<core::DepthBounds>) {
        panic!("not supported");
    }
    fn set_stencil_state(&self, value: &StencilState) {
        unimplemented!()
    }
    fn set_viewport(&self, value: &core::Viewport) {
        unimplemented!()
    }
    fn set_scissor_rect(&self, value: &core::Rect2D<u32>) {
        unimplemented!()
    }
    fn bind_descriptor_sets(
        &self,
        pipeline_layout: &PipelineLayout,
        start_index: usize,
        descriptor_sets: &[DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        unimplemented!()
    }

    fn bind_vertex_buffers(&self, start_index: usize, buffers: &[(&Buffer, usize)]) {
        unimplemented!()
    }

    fn bind_index_buffer(&self, buffer: &Buffer, offset: usize, format: core::IndexFormat) {
        unimplemented!()
    }

    fn draw(
        &self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        start_instance_index: u32,
    ) {
        unimplemented!()
    }
    fn draw_indexed(
        &self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        index_offset: u32,
        start_instance_index: u32,
    ) {
        unimplemented!()
    }
}

impl core::RenderSubpassCommandEncoder<Backend> for CommandBuffer {
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline) {
        self.expect_graphics_pipeline().bind_graphics_pipeline(
            pipeline,
        )
    }
    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        self.expect_graphics_pipeline().set_blend_constants(value)
    }
    fn set_depth_bias(&mut self, value: Option<core::DepthBias>) {
        self.expect_graphics_pipeline().set_depth_bias(value)
    }
    fn set_depth_bounds(&mut self, value: Option<core::DepthBounds>) {
        self.expect_graphics_pipeline().set_depth_bounds(value)
    }
    fn set_stencil_state(&mut self, value: &StencilState) {
        self.expect_graphics_pipeline().set_stencil_state(value)
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        self.expect_graphics_pipeline().set_viewport(value)
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        self.expect_graphics_pipeline().set_scissor_rect(value)
    }
    fn bind_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout,
        start_index: usize,
        descriptor_sets: &[DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        self.expect_graphics_pipeline().bind_descriptor_sets(
            pipeline_layout,
            start_index,
            descriptor_sets,
            dynamic_offsets,
        )
    }

    fn bind_vertex_buffers(&mut self, start_index: usize, buffers: &[(&Buffer, usize)]) {
        self.expect_graphics_pipeline().bind_vertex_buffers(
            start_index,
            buffers,
        )
    }

    fn bind_index_buffer(&mut self, buffer: &Buffer, offset: usize, format: core::IndexFormat) {
        self.expect_graphics_pipeline().bind_index_buffer(
            buffer,
            offset,
            format,
        )
    }

    fn draw(
        &mut self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        start_instance_index: u32,
    ) {
        self.expect_graphics_pipeline().draw(
            num_vertices,
            num_instances,
            start_vertex_index,
            start_instance_index,
        )
    }
    fn draw_indexed(
        &mut self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        index_offset: u32,
        start_instance_index: u32,
    ) {
        self.expect_graphics_pipeline().draw_indexed(
            num_vertices,
            num_instances,
            start_vertex_index,
            index_offset,
            start_instance_index,
        )
    }
}

impl core::ComputeCommandEncoder<Backend> for CommandBuffer {
    fn bind_compute_pipeline(&mut self, pipeline: &ComputePipeline) {
        unimplemented!()
    }
    fn dispatch(&mut self, workgroup_count: Vector3<u32>) {
        unimplemented!()
    }
}

impl core::BlitCommandEncoder<Backend> for CommandBuffer {}

impl SecondaryCommandBuffer {
    fn new(encoder: RenderCommandEncoder) -> Self {
        Self { encoder }
    }
}

unsafe impl Send for SecondaryCommandBuffer {}

impl core::SecondaryCommandBuffer<Backend> for SecondaryCommandBuffer {
    fn end_encoding(&mut self) {
        self.encoder.metal_encoder.end_encoding();
    }
}

impl core::Marker for SecondaryCommandBuffer {
    fn set_label(&self, label: Option<&str>) {
        self.encoder.metal_encoder.set_label(label.unwrap_or(""));
    }
}

impl core::RenderSubpassCommandEncoder<Backend> for SecondaryCommandBuffer {
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline) {
        self.encoder.bind_graphics_pipeline(pipeline)
    }
    fn set_blend_constants(&mut self, value: &[f32; 4]) {
        self.encoder.set_blend_constants(value)
    }
    fn set_depth_bias(&mut self, value: Option<core::DepthBias>) {
        self.encoder.set_depth_bias(value)
    }
    fn set_depth_bounds(&mut self, value: Option<core::DepthBounds>) {
        self.encoder.set_depth_bounds(value)
    }
    fn set_stencil_state(&mut self, value: &StencilState) {
        self.encoder.set_stencil_state(value)
    }
    fn set_viewport(&mut self, value: &core::Viewport) {
        self.encoder.set_viewport(value)
    }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<u32>) {
        self.encoder.set_scissor_rect(value)
    }
    fn bind_descriptor_sets(
        &mut self,
        pipeline_layout: &PipelineLayout,
        start_index: usize,
        descriptor_sets: &[DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        self.encoder.bind_descriptor_sets(
            pipeline_layout,
            start_index,
            descriptor_sets,
            dynamic_offsets,
        )
    }

    fn bind_vertex_buffers(&mut self, start_index: usize, buffers: &[(&Buffer, usize)]) {
        self.encoder.bind_vertex_buffers(start_index, buffers)
    }

    fn bind_index_buffer(&mut self, buffer: &Buffer, offset: usize, format: core::IndexFormat) {
        self.encoder.bind_index_buffer(buffer, offset, format)
    }

    fn draw(
        &mut self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        start_instance_index: u32,
    ) {
        self.encoder.draw(
            num_vertices,
            num_instances,
            start_vertex_index,
            start_instance_index,
        )
    }
    fn draw_indexed(
        &mut self,
        num_vertices: u32,
        num_instances: u32,
        start_vertex_index: u32,
        index_offset: u32,
        start_instance_index: u32,
    ) {
        self.encoder.draw_indexed(
            num_vertices,
            num_instances,
            start_vertex_index,
            index_offset,
            start_instance_index,
        )
    }
}
