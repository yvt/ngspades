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

use cgmath::prelude::*;
use cgmath::{Vector3, Matrix4, Point3};

static SPIRV_FRAG: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/cube.frag.spv"));
static SPIRV_VERT: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/cube.vert.spv"));

static IMAGE: &[u8] = include_bytes!("nyancat.raw");

use core::DebugMarker;

use std::sync::Arc;
use std::{mem, ptr};

#[repr(C)]
struct Vertex([i16; 4] /* position */, [i16; 2] /* uv */);

const CUBE_VERTICES: &[Vertex] = &[
    // +Z
    Vertex([-1, 1, 1, 1], [0, 0]),
    Vertex([1, 1, 1, 1], [1, 0]),
    Vertex([-1, -1, 1, 1], [0, 1]),
    Vertex([1, -1, 1, 1], [1, 1]),

    // -Z
    Vertex([1, 1, -1, 1], [0, 0]),
    Vertex([-1, 1, -1, 1], [1, 0]),
    Vertex([1, -1, -1, 1], [0, 1]),
    Vertex([-1, -1, -1, 1], [1, 1]),

    // +X
    Vertex([1, -1, 1, 1], [0, 0]),
    Vertex([1, 1, 1, 1], [1, 0]),
    Vertex([1, -1, -1, 1], [0, 1]),
    Vertex([1, 1, -1, 1], [1, 1]),

    // -X
    Vertex([-1, -1, -1, 1], [0, 0]),
    Vertex([-1, 1, -1, 1], [1, 0]),
    Vertex([-1, -1, 1, 1], [0, 1]),
    Vertex([-1, 1, 1, 1], [1, 1]),

    // +Y
    Vertex([-1, 1, -1, 1], [0, 0]),
    Vertex([1, 1, -1, 1], [1, 0]),
    Vertex([-1, 1, 1, 1], [0, 1]),
    Vertex([1, 1, 1, 1], [1, 1]),

    // -Y
    Vertex([-1, -1, 1, 1], [0, 0]),
    Vertex([1, -1, 1, 1], [1, 0]),
    Vertex([-1, -1, -1, 1], [0, 1]),
    Vertex([1, -1, -1, 1], [1, 1]),
];
const CUBE_INDICES: &[[u32; 5]] = &[
    [0, 1, 2, 3, 0xffffffff],
    [4, 5, 6, 7, 0xffffffff],
    [8, 9, 10, 11, 0xffffffff],
    [12, 13, 14, 15, 0xffffffff],
    [16, 17, 18, 19, 0xffffffff],
    [20, 21, 22, 23, 0xffffffff],
];

const VERTEX_ATTRIBUTE_POSITION: core::VertexAttributeLocation = 0;
const VERTEX_ATTRIBUTE_UV: core::VertexAttributeLocation = 1;

#[repr(C)]
struct SceneParams {
    view_proj_matrix: Matrix4<f32>,
}

#[repr(C)]
struct ObjectParams {
    model_matrix: Matrix4<f32>,
}

/// Type alias (because I tired of copying and pasting
/// `<B::UniversalHeap as core::MappableHeap>::Allocation`)
struct UHAlloc<B: Backend>(<B::UniversalHeap as core::MappableHeap>::Allocation);

struct MyApp<B: Backend> {
    device: Arc<B::Device>,
    heap: B::UniversalHeap,
    last_drawable_info: DrawableInfo,
    render_pass: B::RenderPass,
    depth_buffer: (B::Image, B::ImageView, UHAlloc<B>, core::ImageFormat),

    pipeline_layout: B::PipelineLayout,
    pipeline: B::GraphicsPipeline,
    index_buffer: B::Buffer,
    vertex_buffer: B::Buffer,
    descriptor_set: B::DescriptorSet,
    scene_params: (B::Buffer, UHAlloc<B>),
    obj_params_staging: (B::Buffer, UHAlloc<B>),
    obj_params: B::Buffer,

    command_buffer: B::CommandBuffer,
    obj_params_fence: B::Fence,
}

impl<B: Backend> MyApp<B> {
    fn new<W: Window<Backend = B>>(w: &W) -> Self {
        let device = w.device().clone();
        let drawable_info = w.swapchain().drawable_info();

        let mut heap = device.factory().make_universal_heap().unwrap();
        let vertex_buffer = DeviceUtils::<B>::new(&device)
            .make_preinitialized_buffer(
                &mut heap,
                CUBE_VERTICES,
                core::BufferUsage::VertexBuffer.into(),
                core::PipelineStage::VertexInput.into(),
                core::AccessType::VertexAttributeRead.into(),
                core::DeviceEngine::Universal,
            )
            .0;
        vertex_buffer.set_label(Some("cube vertex buffer"));
        let index_buffer = DeviceUtils::<B>::new(&device)
            .make_preinitialized_buffer(
                &mut heap,
                CUBE_INDICES,
                core::BufferUsage::IndexBuffer.into(),
                core::PipelineStage::VertexInput.into(),
                core::AccessType::IndexRead.into(),
                core::DeviceEngine::Universal,
            )
            .0;
        index_buffer.set_label(Some("cube index buffer"));
        let depth_buffer = Self::make_depth_buffer(&device, &mut heap, &drawable_info);
        let render_pass = Self::make_render_pass(&device, drawable_info.format, depth_buffer.3);

        let dsl_desc = core::DescriptorSetLayoutDescription {
            bindings: &[
                // SceneParams u_scene_params
                core::DescriptorSetLayoutBinding {
                    location: 0,
                    descriptor_type: core::DescriptorType::ConstantBuffer,
                    num_elements: 1,
                    stage_flags: core::ShaderStage::Vertex.into(),
                    immutable_samplers: None,
                },
                // ObjectParams u_obj_params
                core::DescriptorSetLayoutBinding {
                    location: 1,
                    descriptor_type: core::DescriptorType::ConstantBuffer,
                    num_elements: 1,
                    stage_flags: core::ShaderStage::Vertex.into(),
                    immutable_samplers: None,
                },
                // sampler2D u_texture
                core::DescriptorSetLayoutBinding {
                    location: 2,
                    descriptor_type: core::DescriptorType::CombinedImageSampler,
                    num_elements: 1,
                    stage_flags: core::ShaderStage::Fragment.into(),
                    immutable_samplers: None,
                },
            ],
        };
        let descriptor_set_layout = device
            .factory()
            .make_descriptor_set_layout(&dsl_desc)
            .unwrap();

        let pipeline_layout = {
            let layout_desc = core::PipelineLayoutDescription {
                descriptor_set_layouts: &[&descriptor_set_layout],
            };
            device.factory().make_pipeline_layout(&layout_desc).unwrap()
        };

        let pipeline = Self::make_pipeline(&device, &render_pass, &pipeline_layout);

        // `SceneParams` uniform buffer
        let scene_params = Self::make_scene_params(&device, &mut heap, &drawable_info);
        scene_params.0.set_label(Some("scene params"));

        // `ObjectParams` uniform buffer
        let obj_param_size = mem::size_of::<ObjectParams>() as core::DeviceSize;
        let obj_params_staging = heap.make_buffer(&core::BufferDescription {
            usage: core::BufferUsage::TransferSource.into(),
            size: obj_param_size,
            storage_mode: core::StorageMode::Shared,
        }).expect("failed to create object param staging buffer")
            .unwrap();
        let obj_params_staging = (obj_params_staging.1, UHAlloc(obj_params_staging.0));
        let obj_params = heap.make_buffer(&core::BufferDescription {
            usage: core::BufferUsage::TransferDestination | core::BufferUsage::UniformBuffer,
            size: obj_param_size,
            storage_mode: core::StorageMode::Private,
        }).expect("failed to create object param buffer")
            .unwrap()
            .1;
        obj_params.set_label(Some("object params"));

        // Create image
        let (image, image_view, _) = DeviceUtils::<B>::new(&device).make_preinitialized_image_no_mip(
            &mut heap,
            IMAGE,
            core::ImageDescription {
                usage: core::ImageUsage::Sampled.into(),
                format: core::ImageFormat::SrgbRgba8,
                extent: Vector3::new(128, 128, 1),
                ..Default::default()
            },
            core::PipelineStage::FragmentShader
                .into(),
            core::AccessType::ShaderRead.into(),
            core::ImageLayout::ShaderRead,
            core::DeviceEngine::Universal,
        );
        image.set_label(Some("cube texture"));
        image_view.set_label(Some("cube texture image view"));

        // Create and initialize a descriptor set
        let mut ds_pool = device
            .factory()
            .make_descriptor_pool(&core::DescriptorPoolDescription {
                max_num_sets: 1,
                supports_deallocation: false,
                pool_sizes: &[
                    core::DescriptorPoolSize {
                        descriptor_type: core::DescriptorType::ConstantBuffer,
                        num_descriptors: 2,
                    },
                    core::DescriptorPoolSize {
                        descriptor_type: core::DescriptorType::CombinedImageSampler,
                        num_descriptors: 1,
                    },
                ],
            })
            .unwrap();
        let descriptor_set = ds_pool
            .make_descriptor_set(&core::DescriptorSetDescription {
                layout: &descriptor_set_layout,
            })
            .unwrap()
            .unwrap()
            .0;
        descriptor_set.update(
            &[
                core::WriteDescriptorSet {
                    start_binding: 0,
                    start_index: 0,
                    elements: core::WriteDescriptors::ConstantBuffer(
                        &[
                            core::DescriptorBuffer {
                                buffer: &scene_params.0,
                                offset: 0,
                                range: mem::size_of::<SceneParams>() as core::DeviceSize,
                            },
                            core::DescriptorBuffer {
                                buffer: &obj_params,
                                offset: 0,
                                range: obj_param_size,
                            },
                        ],
                    ),
                },
            ],
        );
        let sampler = device
            .factory()
            .make_sampler(&core::SamplerDescription {
                mag_filter: core::Filter::Linear,
                min_filter: core::Filter::Linear,
                mipmap_mode: core::MipmapMode::Linear,
                address_mode: [core::SamplerAddressMode::Repeat; 3],
                lod_min_clamp: 0f32,
                lod_max_clamp: 0.5f32,
                max_anisotropy: 1,
                compare_function: None,
                border_color: core::SamplerBorderColor::FloatOpaqueBlack,
                unnormalized_coordinates: false,
            })
            .unwrap(); // TODO: use immutable sampler
        descriptor_set.update(
            &[
                core::WriteDescriptorSet {
                    start_binding: 2,
                    start_index: 0,
                    elements: core::WriteDescriptors::CombinedImageSampler(
                        &[
                            (
                                core::DescriptorImage {
                                    image_view: &image_view,
                                    image_layout: core::ImageLayout::ShaderRead,
                                },
                                &sampler,
                            ),
                        ],
                    ),
                },
            ],
        );

        let obj_params_fence = device
            .main_queue()
            .make_fence(&core::FenceDescription {
                update_engines: core::DeviceEngine::Copy.into(),
                wait_engines: core::DeviceEngine::Universal.into(),
            })
            .unwrap();

        let command_buffer = device.main_queue().make_command_buffer().unwrap();

        command_buffer.set_label(Some("main primary command buffer"));

        Self {
            device,
            heap,
            last_drawable_info: drawable_info.clone(),
            render_pass,
            depth_buffer,

            pipeline_layout,
            pipeline,
            index_buffer,
            vertex_buffer,
            descriptor_set,
            scene_params,
            obj_params_staging,
            obj_params,

            command_buffer,
            obj_params_fence,
        }
    }

    fn make_scene_params(
        device: &B::Device,
        heap: &mut B::UniversalHeap,
        drawable_info: &DrawableInfo,
    ) -> (B::Buffer, UHAlloc<B>) {
        let proj_mat = cgmath::perspective(
            cgmath::Deg(60f32),
            drawable_info.extents.x as f32 / drawable_info.extents.y as f32,
            1f32,
            100f32,
        );

        // Convert the range of Z coordinate of [-1, 1] (OpenGL) to match
        // [0, 1] (Vulkan)
        let proj_mat = Matrix4::from_translation(Vector3::new(0f32, 0f32, 0.5f32)) *
            Matrix4::from_nonuniform_scale(1f32, 1f32, 0.5f32) * proj_mat;

        let view_mat = Matrix4::look_at(
            Point3::new(0f32, 0f32, -4f32),
            Point3::new(0f32, 0f32, 0f32),
            Vector3::new(0f32, 1f32, 0f32),
        );

        let scene_params = SceneParams { view_proj_matrix: proj_mat * view_mat };

        let (buffer, alloc) = DeviceUtils::<B>::new(&device).make_preinitialized_buffer(
            heap,
            &[scene_params],
            core::BufferUsage::UniformBuffer.into(),
            core::PipelineStage::VertexShader
                .into(),
            core::AccessType::ShaderRead.into(),
            core::DeviceEngine::Universal,
        );

        (buffer, UHAlloc(alloc))
    }

    fn make_depth_buffer(
        device: &B::Device,
        heap: &mut B::UniversalHeap,
        drawable_info: &DrawableInfo,
    ) -> (B::Image, B::ImageView, UHAlloc<B>, core::ImageFormat) {
        let &format = [core::ImageFormat::DepthFloat32, core::ImageFormat::Depth24]
            .iter()
            .filter(|&fmt| {
                device
                    .capabilities()
                    .image_format_features(*fmt, core::ImageTiling::Optimal)
                    .contains(core::ImageFormatFeature::DepthStencilAttachment)
            })
            .nth(0)
            .expect(
                "The device does not support any of required \
                          formats",
            );
        let i_desc = core::ImageDescription {
            usage: core::ImageUsage::DepthStencilAttachment.into(),
            format,
            extent: drawable_info.extents,
            ..Default::default()
        };
        let (alloc, image) = heap.make_image(&i_desc)
            .expect("failed to create the depth buffer image")
            .unwrap();

        image.set_label(Some("depth buffer image"));

        let image_view = {
            let iv_desc = core::ImageViewDescription {
                image_type: core::ImageType::TwoD,
                image: &image,
                format,
                range: core::ImageSubresourceRange::default(),
            };
            device.factory().make_image_view(&iv_desc).expect(
                "failed to create the depth buffer image view",
            )
        };

        image_view.set_label(Some("depth buffer image view"));

        (image, image_view, UHAlloc(alloc), format)
    }

    fn make_render_pass(
        device: &B::Device,
        drawable_format: core::ImageFormat,
        depth_format: core::ImageFormat,
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
                    final_layout: core::ImageLayout::Present,
                },
                core::RenderPassAttachmentDescription {
                    may_alias: false,
                    format: depth_format,
                    load_op: core::AttachmentLoadOp::Clear,
                    store_op: core::AttachmentStoreOp::DontCare,
                    stencil_load_op: core::AttachmentLoadOp::DontCare,
                    stencil_store_op: core::AttachmentStoreOp::DontCare,
                    initial_layout: core::ImageLayout::Undefined,
                    final_layout: core::ImageLayout::DepthStencilAttachment,
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
                    depth_stencil_attachment: Some(core::RenderPassAttachmentReference {
                        attachment_index: Some(1),
                        layout: core::ImageLayout::DepthStencilAttachment,
                    }),
                    preserve_attachments: &[],
                },
            ],
            dependencies: &[],
        };

        let render_pass = factory.make_render_pass(&desc).unwrap();
        render_pass.set_label(Some("main render pass"));
        render_pass
    }

    fn make_pipeline(
        device: &B::Device,
        render_pass: &B::RenderPass,
        pipeline_layout: &B::PipelineLayout,
    ) -> B::GraphicsPipeline {
        let factory = device.factory();

        let vertex_shader_desc =
            core::ShaderModuleDescription { spirv_code: SPIRV_VERT.as_u32_slice() };
        let vertex_shader = factory.make_shader_module(&vertex_shader_desc).unwrap();

        let fragment_shader_desc =
            core::ShaderModuleDescription { spirv_code: SPIRV_FRAG.as_u32_slice() };
        let fragment_shader = factory.make_shader_module(&fragment_shader_desc).unwrap();

        use core::{VertexFormat, ScalarFormat};
        use core::Signedness::*;
        use core::Normalizedness::*;
        use core::VectorWidth::*;

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
                    format: VertexFormat(Vector4, ScalarFormat::I16(Signed, Unnormalized)),
                    offset: 0,
                },
                core::VertexAttributeDescription {
                    location: VERTEX_ATTRIBUTE_UV,
                    binding: 0,
                    format: VertexFormat(Vector2, ScalarFormat::I16(Signed, Unnormalized)),
                    offset: 8,
                },
            ],
            topology: core::PrimitiveTopology::TriangleStrip,
            rasterizer: Some(core::GraphicsPipelineRasterizerDescription {
                viewport: core::StaticOrDynamic::Dynamic,
                color_attachments,
                ..Default::default()
            }),
            pipeline_layout,
            render_pass,
            subpass_index: 0,
        };

        factory.make_graphics_pipeline(&desc).unwrap()
    }

    fn update_object_params(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now();
        let delta = now.duration_since(UNIX_EPOCH).unwrap();
        let delta = delta.checked_div(5).unwrap();
        let params = ObjectParams {
            model_matrix: Matrix4::from_axis_angle(
                Vector3::new(2f32, 1f32, 0f32).normalize(),
                cgmath::Deg(delta.subsec_nanos() as f32 * 360.0e-9f32),
            ),
        };

        let mut map = self.heap
            .map_memory(&mut (self.obj_params_staging.1).0)
            .unwrap();
        unsafe {
            ptr::copy(&params, map.as_mut_ptr() as *mut ObjectParams, 1);
        }
    }
}

impl<B: Backend> App<B> for MyApp<B> {
    fn update_drawable_info(&mut self, drawable_info: &DrawableInfo) {
        if drawable_info.extents != self.last_drawable_info.extents {
            // Recreate the depth buffer
            let new = Self::make_depth_buffer(&self.device, &mut self.heap, drawable_info);
            let old = mem::replace(&mut self.depth_buffer, new);
            self.heap.deallocate((old.2).0);

            // Update the scene params
            let new = Self::make_scene_params(&self.device, &mut self.heap, drawable_info);
            let old = mem::replace(&mut self.scene_params, new);
            self.heap.deallocate((old.1).0);

            self.descriptor_set.update(
                &[
                    core::WriteDescriptorSet {
                        start_binding: 0,
                        start_index: 0,
                        elements: core::WriteDescriptors::ConstantBuffer(
                            &[
                                core::DescriptorBuffer {
                                    buffer: &self.scene_params.0,
                                    offset: 0,
                                    range: mem::size_of::<SceneParams>() as core::DeviceSize,
                                },
                            ],
                        ),
                    },
                ],
            );
        }
        if drawable_info.format != self.last_drawable_info.format {
            self.render_pass =
                Self::make_render_pass(&self.device, drawable_info.format, self.depth_buffer.3);
            self.pipeline =
                Self::make_pipeline(&self.device, &self.render_pass, &self.pipeline_layout);
        }

        self.last_drawable_info = drawable_info.clone();
    }

    fn render_to(&mut self, drawable: &Drawable<Backend = B>, drawable_info: &DrawableInfo) {
        let image_view;
        let framebuffer;
        let viewport;

        {
            let device: &B::Device = &self.device;
            image_view = device
                .factory()
                .make_image_view(&core::ImageViewDescription {
                    image_type: core::ImageType::TwoD,
                    image: drawable.image(),
                    format: drawable_info.format,
                    range: core::ImageSubresourceRange::default(),
                })
                .unwrap();
            framebuffer = device
                .factory()
                .make_framebuffer(&core::FramebufferDescription {
                    render_pass: &self.render_pass,
                    attachments: &[
                        core::FramebufferAttachmentDescription {
                            image_view: &image_view,
                            clear_values: core::ClearValues::ColorFloat([0f32, 0f32, 0f32, 1f32]),
                        },
                        core::FramebufferAttachmentDescription {
                            image_view: &self.depth_buffer.1,
                            clear_values: core::ClearValues::DepthStencil(1f32, 0),
                        },
                    ],
                    width: drawable_info.extents.x,
                    height: drawable_info.extents.y,
                    num_layers: 1,
                })
                .unwrap();
            viewport = core::Viewport {
                x: 0f32,
                y: 0f32,
                width: drawable_info.extents.x as f32,
                height: drawable_info.extents.y as f32,
                min_depth: 0f32,
                max_depth: 1f32,
            };

            // TODO: use multiple buffers
            let ref mut cb = self.command_buffer;
            cb.wait_completion().unwrap();
        }

        self.update_object_params();

        let ref mut cb = self.command_buffer;
        let device: &B::Device = &self.device;

        cb.begin_encoding();

        // Upload the object params
        cb.begin_copy_pass(core::DeviceEngine::Copy);
        {
            let size = mem::size_of::<ObjectParams>() as core::DeviceSize;
            cb.acquire_resource(
                core::PipelineStage::Transfer.into(),
                core::AccessType::TransferRead.into(),
                core::DeviceEngine::Host,
                &core::SubresourceWithLayout::Buffer {
                    buffer: &self.obj_params_staging.0,
                    offset: 0,
                    len: size,
                },
            );
            cb.begin_debug_group(&core::DebugMarker::new("staging to object param buffer"));
            cb.copy_buffer(&self.obj_params_staging.0, 0, &self.obj_params, 0, size);
            cb.end_debug_group();
            cb.release_resource(
                core::PipelineStage::Transfer.into(),
                core::AccessType::TransferWrite.into(),
                core::DeviceEngine::Universal,
                &core::SubresourceWithLayout::Buffer {
                    buffer: &self.obj_params,
                    offset: 0,
                    len: size,
                },
            );
            cb.update_fence(
                core::PipelineStage::Transfer.into(),
                core::AccessType::TransferWrite.into(),
                &self.obj_params_fence,
            );
        }
        cb.end_pass();

        cb.begin_render_pass(&framebuffer, core::DeviceEngine::Universal);
        {
            // Render the scene
            cb.begin_render_subpass(core::RenderPassContents::Inline);
            {
                if let Some(fence) = drawable.acquiring_fence() {
                    cb.wait_fence(
                        core::PipelineStage::ColorAttachmentOutput.into(),
                        core::AccessType::ColorAttachmentWrite.into(),
                        fence,
                    );
                }
                cb.wait_fence(
                    core::PipelineStage::VertexShader.into(),
                    core::AccessType::ShaderRead.into(),
                    &self.obj_params_fence,
                );

                cb.begin_debug_group(&DebugMarker::new("render a cube"));
                cb.bind_graphics_pipeline(&self.pipeline);
                cb.set_viewport(&viewport);
                cb.bind_graphics_descriptor_sets(
                    &self.pipeline_layout,
                    0,
                    &[&self.descriptor_set],
                    &[],
                );
                cb.bind_vertex_buffers(0, &[(&self.vertex_buffer, 0)]);
                cb.bind_index_buffer(&self.index_buffer, 0, core::IndexFormat::U32);
                cb.draw_indexed(0..(mem::size_of_val(CUBE_INDICES) / 4) as u32, 0, 0..1);
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
