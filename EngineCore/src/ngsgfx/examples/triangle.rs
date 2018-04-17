//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

// Based on Sascha Williems' "triangle.c" Vulkan example (which is licensed under MIT).
// https://github.com/SaschaWillems/Vulkan/blob/master/triangle/triangle.cpp

extern crate ngsgfx as gfx;
extern crate cgmath;
#[macro_use]
extern crate include_data;

mod common;
use common::*;

static SPIRV_FRAG: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/triangle.frag.spv"));
static SPIRV_VERT: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/triangle.vert.spv"));

use core::{VertexFormat, VectorWidth, ScalarFormat, DebugMarker};

use std::sync::Arc;
use std::mem;

#[repr(C)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

const VERTEX_ATTRIBUTE_POSITION: core::VertexAttributeLocation = 0;
const VERTEX_ATTRIBUTE_COLOR: core::VertexAttributeLocation = 1;

struct MyApp<B: Backend> {
    device: Arc<B::Device>,
    last_drawable_info: DrawableInfo,
    vertex_buffer: B::Buffer,
    pipeline: B::GraphicsPipeline,
    render_pass: B::RenderPass,
    command_buffer: B::CommandBuffer,
}

impl<B: Backend> MyApp<B> {
    fn new<W: Window<Backend = B>>(w: &W) -> Self {
        let device = w.device().clone();
        let drawable_info = w.swapchain().drawable_info();

        let mut heap = device.factory().make_universal_heap().unwrap();
        let vertex_buffer = Self::make_vertex_buffer(&device, &mut heap);
        let render_pass = Self::make_render_pass(&device, drawable_info.format);
        let pipeline = Self::make_pipeline(&device, &render_pass);
        let command_buffer = device.main_queue().make_command_buffer().unwrap();

        render_pass.set_label(Some("main render pass"));
        command_buffer.set_label(Some("main primary command buffer"));

        Self {
            device,
            last_drawable_info: drawable_info,
            vertex_buffer,
            pipeline,
            render_pass,
            command_buffer,
        }
    }

    fn make_render_pass(device: &B::Device, drawable_format: core::ImageFormat) -> B::RenderPass {
        let factory = device.factory();

        let desc = core::RenderPassDescription {
            attachments: &[
                core::RenderPassAttachmentDescription {
                    may_alias: false,
                    format: drawable_format,
                    load_op: core::AttachmentLoadOp::Clear,
                    store_op: core::AttachmentStoreOp::Store,
                    stencil_load_op: core::AttachmentLoadOp::DontCare,
                    stencil_store_op: core::AttachmentStoreOp::DontCare,
                    initial_layout: core::ImageLayout::Undefined,
                    final_layout: core::ImageLayout::Present,
                },
            ],
            subpasses: &[
                core::RenderSubpassDescription {
                    input_attachments: &[],
                    color_attachments: &[
                        core::RenderPassAttachmentReference {
                            attachment_index: Some(0),
                            layout: core::ImageLayout::ColorAttachment,
                        },
                    ],
                    depth_stencil_attachment: None,
                    preserve_attachments: &[],
                },
            ],
            dependencies: &[],
        };

        factory.make_render_pass(&desc).unwrap()
    }

    fn make_pipeline(device: &B::Device, render_pass: &B::RenderPass) -> B::GraphicsPipeline {
        let factory = device.factory();

        let vertex_shader_desc =
            core::ShaderModuleDescription { spirv_code: SPIRV_VERT.as_u32_slice() };
        let vertex_shader = factory.make_shader_module(&vertex_shader_desc).unwrap();

        let fragment_shader_desc =
            core::ShaderModuleDescription { spirv_code: SPIRV_FRAG.as_u32_slice() };
        let fragment_shader = factory.make_shader_module(&fragment_shader_desc).unwrap();

        let layout_desc = core::PipelineLayoutDescription { descriptor_set_layouts: &[] };
        let layout = factory.make_pipeline_layout(&layout_desc).unwrap();

        let color_attachments = &[Default::default()];
        let desc = core::GraphicsPipelineDescription {
            label: Some("main graphics pipeline"),
            shader_stages: &[
                core::ShaderStageDescription {
                    stage: core::ShaderStage::Fragment,
                    module: &fragment_shader,
                    entry_point_name: "main",
                },
                core::ShaderStageDescription {
                    stage: core::ShaderStage::Vertex,
                    module: &vertex_shader,
                    entry_point_name: "main",
                },
            ],
            vertex_buffers: &[
                core::VertexBufferLayoutDescription {
                    binding: 0,
                    stride: mem::size_of::<Vertex>() as u32,
                    input_rate: core::VertexInputRate::Vertex,
                },
            ],
            vertex_attributes: &[
                core::VertexAttributeDescription {
                    location: VERTEX_ATTRIBUTE_POSITION,
                    binding: 0,
                    format: VertexFormat(VectorWidth::Vector3, ScalarFormat::F32),
                    offset: 0,
                },
                core::VertexAttributeDescription {
                    location: VERTEX_ATTRIBUTE_COLOR,
                    binding: 0,
                    format: VertexFormat(VectorWidth::Vector3, ScalarFormat::F32),
                    offset: 12,
                },
            ],
            topology: core::PrimitiveTopology::Triangles,
            rasterizer: Some(core::GraphicsPipelineRasterizerDescription {
                viewport: core::StaticOrDynamic::Dynamic,
                cull_mode: core::CullMode::None,
                depth_write: false,
                depth_test: core::CompareFunction::Always,
                color_attachments,
                ..Default::default()
            }),
            pipeline_layout: &layout,
            render_pass,
            subpass_index: 0,
        };

        factory.make_graphics_pipeline(&desc).unwrap()
    }

    fn make_vertex_buffer(device: &B::Device, heap: &mut B::UniversalHeap) -> B::Buffer {
        let vertices = [
            Vertex {
                position: [-0.5f32, 0.5f32, 0f32],
                color: [1f32, 0f32, 0f32],
            },
            Vertex {
                position: [0.5f32, 0.5f32, 0f32],
                color: [0f32, 1f32, 0f32],
            },
            Vertex {
                position: [0f32, -0.5f32, 0f32],
                color: [0f32, 0f32, 1f32],
            },
        ];

        DeviceUtils::<B>::new(device)
            .make_preinitialized_buffer(
                heap,
                &vertices,
                core::BufferUsage::VertexBuffer.into(),
                core::PipelineStage::VertexInput.into(),
                core::AccessType::VertexAttributeRead.into(),
                core::DeviceEngine::Universal,
            )
            .0
    }
}

impl<B: Backend> App<B> for MyApp<B> {
    fn update_drawable_info(&mut self, drawable_info: &DrawableInfo) {
        if drawable_info.format != self.last_drawable_info.format {
            self.render_pass = Self::make_render_pass(&self.device, drawable_info.format);
            self.pipeline = Self::make_pipeline(&self.device, &self.render_pass);
        }

        self.last_drawable_info = drawable_info.clone();
    }

    fn render_to(&mut self, drawable: &Drawable<Backend = B>, drawable_info: &DrawableInfo) {
        let device: &B::Device = &self.device;
        let image_view = device
            .factory()
            .make_image_view(&core::ImageViewDescription {
                image_type: core::ImageType::TwoD,
                image: drawable.image(),
                format: drawable_info.format,
                range: core::ImageSubresourceRange::default(),
            })
            .unwrap();
        let framebuffer = device
            .factory()
            .make_framebuffer(&core::FramebufferDescription {
                render_pass: &self.render_pass,
                attachments: &[
                    core::FramebufferAttachmentDescription {
                        image_view: &image_view,
                        clear_values: core::ClearValues::ColorFloat([0f32, 0f32, 0f32, 1f32]),
                    },
                ],
                width: drawable_info.extents.x,
                height: drawable_info.extents.y,
                num_layers: 1,
            })
            .unwrap();
        let viewport = core::Viewport {
            x: 0f32,
            y: 0f32,
            width: drawable_info.extents.x as f32,
            height: drawable_info.extents.y as f32,
            min_depth: 0f32,
            max_depth: 1f32,
        };

        let ref mut cb = self.command_buffer;

        // TODO: use multiple buffers
        cb.wait_completion().unwrap();

        cb.begin_encoding();

        cb.begin_render_pass(&framebuffer, core::DeviceEngine::Universal);
        {
            cb.begin_render_subpass(core::RenderPassContents::Inline);
            {
                if let Some(fence) = drawable.acquiring_fence() {
                    cb.wait_fence(
                        core::PipelineStage::ColorAttachmentOutput.into(),
                        core::AccessType::ColorAttachmentWrite.into(),
                        fence,
                    );
                }
                cb.begin_debug_group(&DebugMarker::new("render a triangle"));
                cb.bind_graphics_pipeline(&self.pipeline);
                cb.set_viewport(&viewport);
                cb.bind_vertex_buffers(0, &[(&self.vertex_buffer, 0)]);
                cb.draw(0..3, 0..1);
                cb.end_debug_group();
                if let Some(fence) = drawable.releasing_fence() {
                    cb.update_fence(
                        core::PipelineStage::ColorAttachmentOutput.into(),
                        core::AccessType::ColorAttachmentWrite.into(),
                        fence,
                    );
                }
            }
            cb.end_render_subpass();

            drawable.finalize(
                cb,
                core::PipelineStage::ColorAttachmentOutput.into(),
                core::AccessType::ColorAttachmentWrite.into(),
                core::ImageLayout::Present,
            );
        }
        cb.end_pass();

        cb.end_encoding().unwrap();

        device
            .main_queue()
            .submit_commands(&mut [&mut *cb], None)
            .unwrap();
        drawable.present();
    }

    fn wait_completion(&mut self) {
        self.command_buffer.wait_completion().unwrap();
    }
}

struct MyAppFactory;

impl AppFactory for MyAppFactory {
    fn run<W: Window>(w: &W) -> Box<App<W::Backend>> {
        Box::new(MyApp::new(w))
    }
}

fn main() {
    run_example::<MyAppFactory>();
}
