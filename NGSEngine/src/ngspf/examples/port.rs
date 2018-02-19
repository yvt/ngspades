//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate atomic_refcell;
extern crate cgmath;
#[macro_use]
extern crate include_data;
extern crate ngsgfx as gfx;
extern crate ngspf;
extern crate refeq;
use gfx::core;

use std::thread;
use std::sync::{mpsc, Arc, Mutex};

use cgmath::{Matrix4, Point2, vec3};
use cgmath::prelude::*;

use refeq::RefEqArc;

use ngspf::viewport::{LayerBuilder, LayerContents, RootRef, VirtualKeyCode, WindowBuilder,
                      WindowEvent, WindowFlagsBit, WindowRef, Workspace};
use ngspf::prelude::*;
use ngspf::ngsbase::Box2;
use ngspf::ngsbase::prelude::*;

mod common;

mod triangle {

    use include_data;

    static SPIRV_FRAG: include_data::DataView =
        include_data!(concat!(env!("OUT_DIR"), "/triangle.frag.spv"));
    static SPIRV_VERT: include_data::DataView =
        include_data!(concat!(env!("OUT_DIR"), "/triangle.vert.spv"));

    use {cgmath, core};
    use core::{Backend, DebugMarker, ScalarFormat, VectorWidth, VertexFormat};
    use gfx::backends::DefaultBackend;
    use gfx::prelude::*;
    use atomic_refcell::AtomicRefCell;

    use cgmath::vec2;

    use std::sync::Arc;
    use std::mem;

    use ngspf::context::PresenterFrame;
    use ngspf::viewport::{Port, PortInstance, PortMountContext, PortRenderContext};

    use common::*;

    #[repr(C)]
    struct Vertex {
        position: [f32; 3],
        color: [f32; 3],
    }

    const VERTEX_ATTRIBUTE_POSITION: core::VertexAttributeLocation = 0;
    const VERTEX_ATTRIBUTE_COLOR: core::VertexAttributeLocation = 1;

    #[derive(Debug)]
    pub struct MyPort;

    impl Port for MyPort {
        fn mount(&self, context: &mut PortMountContext) {
            if let Some(mut context) = context.downcast_mut::<DefaultBackend>() {
                let inst = MyPortInstance::new(context.workspace_device().objects().gfx_device());
                context.set_instance(Box::new(inst));
            } else {
                panic!("Unknown backend");
            }
        }
    }

    #[derive(Debug)]
    struct MyPortInstance<B: Backend> {
        device: Arc<B::Device>,
        vertex_buffer: B::Buffer,
        pipeline: B::GraphicsPipeline,
        render_pass: B::RenderPass,
        command_buffer: Arc<AtomicRefCell<B::CommandBuffer>>,

        rt_extents: cgmath::Vector2<u32>,
        rt_image: B::Image,
        rt_image_view: B::ImageView,
        rt_framebuffer: B::Framebuffer,
    }

    impl<B: Backend> MyPortInstance<B> {
        fn new(device: &Arc<B::Device>) -> Self {
            let mut heap = device.factory().make_universal_heap().unwrap();

            let rt_extents = vec2::<u32>(256, 256);
            let rt_format = core::ImageFormat::SrgbRgba8;

            let vertex_buffer = Self::make_vertex_buffer(&device, &mut heap);
            let render_pass = Self::make_render_pass(&device, rt_format);
            let pipeline = Self::make_pipeline(&device, &render_pass);
            let command_buffer = device
                .main_queue()
                .make_command_buffer()
                .map(AtomicRefCell::new)
                .map(Arc::new)
                .unwrap();

            let rt_image = heap.make_image(&core::ImageDescription {
                usage: core::ImageUsage::Sampled | core::ImageUsage::ColorAttachment,
                format: rt_format,
                extent: rt_extents.extend(1),
                ..Default::default()
            }).unwrap()
                .unwrap()
                .1;

            let rt_image_view = device
                .factory()
                .make_image_view(&core::ImageViewDescription {
                    image_type: core::ImageType::TwoD,
                    image: &rt_image,
                    format: rt_format,
                    range: core::ImageSubresourceRange::default(),
                })
                .unwrap();

            let rt_framebuffer = device
                .factory()
                .make_framebuffer(&core::FramebufferDescription {
                    render_pass: &render_pass,
                    attachments: &[
                        core::FramebufferAttachmentDescription {
                            image_view: &rt_image_view,
                            clear_values: core::ClearValues::ColorFloat([0f32, 0f32, 0f32, 1f32]),
                        },
                    ],
                    width: rt_extents.x,
                    height: rt_extents.y,
                    num_layers: 1,
                })
                .unwrap();

            render_pass.set_label(Some("main render pass"));
            command_buffer
                .borrow()
                .set_label(Some("main primary command buffer"));

            Self {
                device: Arc::clone(device),
                vertex_buffer,
                pipeline,
                render_pass,
                command_buffer,
                rt_extents,
                rt_image,
                rt_image_view,
                rt_framebuffer,
            }
        }

        fn make_render_pass(
            device: &B::Device,
            drawable_format: core::ImageFormat,
        ) -> B::RenderPass {
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
                        final_layout: core::ImageLayout::ShaderRead,
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

            let vertex_shader_desc = core::ShaderModuleDescription {
                spirv_code: SPIRV_VERT.as_u32_slice(),
            };
            let vertex_shader = factory.make_shader_module(&vertex_shader_desc).unwrap();

            let fragment_shader_desc = core::ShaderModuleDescription {
                spirv_code: SPIRV_FRAG.as_u32_slice(),
            };
            let fragment_shader = factory.make_shader_module(&fragment_shader_desc).unwrap();

            let layout_desc = core::PipelineLayoutDescription {
                descriptor_set_layouts: &[],
            };
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

    impl<B: Backend> PortInstance<B> for MyPortInstance<B> {
        fn render(
            &mut self,
            context: &mut PortRenderContext<B>,
            frame: &PresenterFrame,
        ) -> B::ImageView {
            let ref rt_extents = self.rt_extents;
            let viewport = core::Viewport {
                x: 0f32,
                y: 0f32,
                width: rt_extents.x as f32,
                height: rt_extents.y as f32,
                min_depth: 0f32,
                max_depth: 1f32,
            };

            {
                let mut cb = self.command_buffer.borrow_mut();

                // TODO: use multiple buffers
                cb.wait_completion().unwrap();

                cb.begin_encoding();

                cb.begin_render_pass(&self.rt_framebuffer, core::DeviceEngine::Universal);
                {
                    cb.begin_render_subpass(core::RenderPassContents::Inline);
                    {
                        cb.begin_debug_group(&DebugMarker::new("render a triangle"));
                        cb.bind_graphics_pipeline(&self.pipeline);
                        cb.set_viewport(&viewport);
                        cb.bind_vertex_buffers(0, &[(&self.vertex_buffer, 0)]);
                        cb.draw(0..3, 0..1);
                        cb.end_debug_group();
                    }
                    cb.end_render_subpass();
                }
                cb.end_pass();

                cb.end_encoding().unwrap();
            }

            context
                .command_buffers
                .push(Arc::clone(&self.command_buffer));
            context.schedule_next_frame = true;

            self.rt_image_view.clone()
        }
    }
}

fn main() {
    let mut ws = Workspace::new().expect("failed to create a workspace");
    let context = Arc::clone(ws.context());
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);

    // Produce the first frame
    let root = RootRef::clone(ws.root());
    let window: WindowRef;
    {
        let image = LayerBuilder::new()
            .contents(LayerContents::Port(RefEqArc::new(::triangle::MyPort)))
            .bounds(Box2::new(Point2::origin(), Point2::new(512.0, 512.0)))
            .transform(Matrix4::from_translation(vec3(10.0, 10.0, 0.0)))
            .build(&context);

        window = WindowBuilder::new()
            .flags(WindowFlagsBit::Resizable)
            .child(Some(image.into_node_ref()))
            .listener(Some(Box::new(move |event| {
                // Send the event to the producer loop
                let _ = tx.lock().unwrap().send(event.clone());
            })))
            .build(&context);

        let mut frame = context
            .lock_producer_frame()
            .expect("failed to acquire a producer frame");
        ws.root()
            .windows()
            .set(&mut frame, Some(window.clone().into_node_ref()))
            .expect("failed to set the value of proeprty 'windows'");
    }
    context.commit().expect("failed to commit a frame");

    // Start the producer loop
    thread::Builder::new()
        .spawn(move || {
            use std::time::Duration;
            let mut i = 0;
            let mut exit = false;
            while !exit {
                i += 1;

                // Process window events
                for event in rx.try_iter() {
                    match event {
                        WindowEvent::Close => {
                            exit = true;
                        }
                        WindowEvent::KeyboardInput(vk, pressed, _) => {
                            if pressed && vk == VirtualKeyCode::Escape {
                                exit = true;
                            }
                        }
                        _ => {}
                    }
                }

                {
                    let mut frame = context
                        .lock_producer_frame()
                        .expect("failed to acquire a producer frame");

                    window
                        .title()
                        .set(&mut frame, format!("frame = {}", i))
                        .unwrap();

                    if exit {
                        root.exit_loop(&mut frame).unwrap();
                    }
                }
                context.commit().expect("failed to commit a frame");
                thread::sleep(Duration::from_millis(15));
            }
        })
        .unwrap();

    // Start the main loop
    ws.enter_main_loop()
        .expect("error occured while running the main loop");
}
