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

static SPIRV_FRAG: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/triangle.frag.spv"));
static SPIRV_VERT: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/triangle.vert.spv"));

use cgmath::Vector2;

use gfx::core;
use gfx::core::{VertexFormat, VectorWidth, ScalarFormat, DebugMarker};
use gfx::prelude::*;
use gfx::wsi::{DefaultWindow, NewWindow, Window, winit, SwapchainDescription, ColorSpace,
               Swapchain, Drawable, SwapchainError};
use gfx::backends::{DefaultEnvironment, DefaultBackend};

use std::sync::Arc;
use std::{mem, ptr};
use std::cell::RefCell;

#[repr(C)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

const VERTEX_ATTRIBUTE_POSITION: core::VertexAttributeLocation = 0;
const VERTEX_ATTRIBUTE_COLOR: core::VertexAttributeLocation = 1;

struct Renderer<B: Backend> {
    device: Arc<B::Device>,
    vertex_buffer: B::Buffer,
    pipeline: B::GraphicsPipeline,
    render_pass: B::RenderPass,
    command_buffer: RefCell<B::CommandBuffer>,
}

struct RendererView<B: Backend> {
    renderer: Arc<Renderer<B>>,
    size: Vector2<u32>,
}

impl<B: Backend> Renderer<B> {
    fn new(device: Arc<B::Device>) -> Self {
        let mut heap = device.factory().make_universal_heap().unwrap();
        let vertex_buffer = Self::make_vertex_buffer(&device, &mut heap);
        let render_pass = Self::make_render_pass(&device);
        let pipeline = Self::make_pipeline(&device, &render_pass);
        let command_buffer = device
            .main_queue()
            .make_command_buffer()
            .map(RefCell::new)
            .unwrap();

        render_pass.set_label(Some("main render pass"));
        command_buffer.borrow().set_label(
            Some("main primary command buffer"),
        );

        Self {
            device,
            vertex_buffer,
            pipeline,
            render_pass,
            command_buffer,
        }
    }

    fn make_render_pass(device: &B::Device) -> B::RenderPass {
        let factory = device.factory();

        let desc = core::RenderPassDescription {
            attachments: &[
                core::RenderPassAttachmentDescription {
                    may_alias: false,
                    format: core::ImageFormat::SrgbBgra8,
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
        let size = mem::size_of_val(&vertices) as core::DeviceSize;
        let staging_buffer_desc = core::BufferDescription {
            usage: core::BufferUsage::TransferSource.into(),
            size,
            storage_mode: core::StorageMode::Shared,
        };
        let buffer_desc = core::BufferDescription {
            usage: core::BufferUsage::VertexBuffer | core::BufferUsage::TransferDestination,
            size,
            storage_mode: core::StorageMode::Private,
        };

        // Create a staging heap/buffer
        let (mut staging_alloc, staging_buffer) =
            heap.make_buffer(&staging_buffer_desc).unwrap().unwrap();
        {
            let mut map = heap.map_memory(&mut staging_alloc).unwrap();
            unsafe {
                ptr::copy(
                    vertices.as_ptr(),
                    map.as_mut_ptr() as *mut Vertex,
                    vertices.len(),
                );
            }
        }

        // Create a device heap/buffer
        let buffer = heap.make_buffer(&buffer_desc).unwrap().unwrap().1;

        // Add debug labels
        buffer.set_label(Some("main vertex buffer"));
        staging_buffer.set_label(Some("staging buffer for main vertex buffer"));

        // Fill the buffer
        let queue = device.main_queue();
        let mut cb = queue.make_command_buffer().unwrap();
        cb.set_label(Some("staging CB to main vertex buffer"));
        cb.begin_encoding();
        cb.begin_copy_pass(core::DeviceEngine::Universal);
        cb.acquire_resource(
            core::PipelineStage::Transfer.into(),
            core::AccessType::TransferRead.into(),
            core::DeviceEngine::Host,
            &core::SubresourceWithLayout::Buffer {
                buffer: &staging_buffer,
                offset: 0,
                len: size,
            },
        );
        cb.begin_debug_group(&DebugMarker::new("staging to main vertex buffer"));
        cb.copy_buffer(&staging_buffer, 0, &buffer, 0, size);
        cb.end_debug_group();
        cb.resource_barrier(
            core::PipelineStage::Transfer.into(),
            core::AccessType::TransferWrite.into(),
            core::PipelineStage::VertexInput.into(),
            core::AccessType::VertexAttributeRead.into(),
            &core::SubresourceWithLayout::Buffer {
                buffer: &buffer,
                offset: 0,
                len: size,
            },
        );
        cb.end_pass();
        cb.end_encoding().unwrap();
        queue.submit_commands(&mut [&mut cb], None).unwrap();
        cb.wait_completion().unwrap();

        // Phew! Done!
        buffer
    }
}

impl<B: Backend> RendererView<B> {
    fn new(renderer: &Arc<Renderer<B>>, size: Vector2<u32>) -> Self {
        Self {
            renderer: renderer.clone(),
            size,
        }
    }

    fn render_to<S>(&self, swapchain: &S) -> Result<(), SwapchainError>
    where
        S: Swapchain<Backend = B>,
    {
        let drawable = swapchain.next_drawable(&gfx::wsi::FrameDescription {
            acquiring_engines: core::DeviceEngine::Universal.into(),
        })?;
        let renderer: &Renderer<B> = &*self.renderer;
        let device: &B::Device = &*renderer.device;
        let image_view = device
            .factory()
            .make_image_view(&core::ImageViewDescription {
                image_type: core::ImageType::TwoD,
                image: drawable.image(),
                format: swapchain.image_format(),
                range: core::ImageSubresourceRange::default(),
            })
            .unwrap();
        let framebuffer = device
            .factory()
            .make_framebuffer(&core::FramebufferDescription {
                render_pass: &renderer.render_pass,
                attachments: &[
                    core::FramebufferAttachmentDescription {
                        image_view: &image_view,
                        clear_values: core::ClearValues::ColorFloat([0f32, 0f32, 0f32, 1f32]),
                    },
                ],
                width: self.size.x,
                height: self.size.y,
                num_layers: 1,
            })
            .unwrap();
        let viewport = core::Viewport {
            x: 0f32,
            y: 0f32,
            width: self.size.x as f32,
            height: self.size.y as f32,
            min_depth: 0f32,
            max_depth: 1f32,
        };

        let mut cb = renderer.command_buffer.borrow_mut();

        // TODO: use multiple buffers
        cb.wait_completion().unwrap();

        cb.begin_encoding();

        cb.begin_render_pass(&framebuffer, core::DeviceEngine::Universal);
        cb.begin_render_subpass(core::RenderPassContents::Inline);
        if let Some(fence) = drawable.acquiring_fence() {
            cb.wait_fence(
                core::PipelineStage::ColorAttachmentOutput.into(),
                core::AccessType::ColorAttachmentWrite.into(),
                fence,
            );
        }
        cb.begin_debug_group(&DebugMarker::new("render a triangle"));
        cb.bind_graphics_pipeline(&renderer.pipeline);
        cb.set_viewport(&viewport);
        cb.bind_vertex_buffers(0, &[(&renderer.vertex_buffer, 0)]);
        cb.draw(0..3, 0..1);
        cb.end_debug_group();

        cb.end_render_subpass();

        drawable.finalize(
            &mut cb,
            core::PipelineStage::ColorAttachmentOutput.into(),
            core::AccessType::ColorAttachmentWrite.into(),
            core::ImageLayout::Present,
        );
        cb.end_pass();

        cb.end_encoding().unwrap();

        device
            .main_queue()
            .submit_commands(&mut [&mut *cb], None)
            .unwrap();
        drawable.present();

        Ok(())
    }
}


struct App<W: Window> {
    window: W,
    renderer: Arc<Renderer<W::Backend>>,
    renderer_view: RefCell<Option<RendererView<W::Backend>>>,
}

fn create_renderer_view<W: Window>(
    renderer: &Arc<Renderer<W::Backend>>,
    window: &W,
) -> RendererView<W::Backend> {
    let extents = window.swapchain().image_extents();
    assert_eq!(extents.z, 1);
    RendererView::new(&renderer, Vector2::new(extents.x, extents.y))
}

impl<W: Window> App<W> {
    fn new(window: W) -> Self {
        let device = window.device().clone();
        let renderer = Arc::new(Renderer::new(device));
        Self {
            renderer_view: RefCell::new(None),
            renderer,
            window,
        }
    }

    fn run(&self, events_loop: &mut winit::EventsLoop) {
        let mut running = true;

        DefaultBackend::autorelease_pool_scope(|mut arp| while running {
            events_loop.poll_events(|event| match event {
                winit::Event::WindowEvent { event: winit::WindowEvent::Closed, .. } => {
                    running = false;
                }
                winit::Event::WindowEvent { event: winit::WindowEvent::Resized(_, _), .. } => {
                    self.update_view();
                }
                _ => (),
            });

            self.update();
            arp.drain();
        });
    }

    fn update_view(&self) {
        self.window.update_swapchain();
        *self.renderer_view.borrow_mut() = Some(create_renderer_view(&self.renderer, &self.window));
    }

    fn update(&self) {
        if self.renderer_view.borrow().is_none() {
            self.update_view();
        }
        let mut ret = self.renderer_view
            .borrow_mut()
            .as_mut()
            .unwrap()
            .render_to(self.window.swapchain());
        if ret == Err(SwapchainError::OutOfDate) {
            self.update_view();
            ret = self.renderer_view
                .borrow_mut()
                .as_mut()
                .unwrap()
                .render_to(self.window.swapchain());
        }
        if let Err(x) = ret {
            println!("failed to acquire the next drawable: {:?}", x);
        }
    }
}

fn main() {
    use gfx::core::{Environment, InstanceBuilder};

    DefaultBackend::autorelease_pool_scope(|_| {
        let mut events_loop = winit::EventsLoop::new();
        let builder = winit::WindowBuilder::new();

        let mut instance_builder = <DefaultEnvironment as Environment>::InstanceBuilder::new()
            .expect("InstanceBuilder::new() have failed");
        DefaultWindow::modify_instance_builder(&mut instance_builder);
        instance_builder.enable_debug_report(
            core::DebugReportType::Information | core::DebugReportType::Warning |
                core::DebugReportType::PerformanceWarning |
                core::DebugReportType::Error,
            gfx::debug::report::TermStdoutDebugReportHandler::new(),
        );
        instance_builder.enable_validation();
        instance_builder.enable_debug_marker();

        let instance = instance_builder.build().expect(
            "InstanceBuilder::build() have failed",
        );

        let sc_desc = SwapchainDescription {
            desired_formats: &[(None, Some(ColorSpace::SrgbNonlinear))],
            image_usage: core::ImageUsage::ColorAttachment.into(),
        };
        let window = DefaultWindow::new(builder, &events_loop, &instance, &sc_desc).unwrap();
        let app = App::new(window);
        app.run(&mut events_loop);
        println!("Exiting...");
    });
}
