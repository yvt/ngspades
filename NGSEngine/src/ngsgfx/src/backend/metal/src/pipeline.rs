//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;

use std::iter::empty;

use {RefEqArc, OCPtr, imp, utils, formats, translate_compare_function};

/// Graphics pipeline state.
///
/// Associated with `MTLRenderPipelineState` and `MTLDepthStencilState` (optionally).
///
/// The layout of Metal argument tables are entirely defined by `PipelineLayout`, except
/// the last zero or more elements of a vertex shader's buffer argument table (or, so called
/// Metal vertex buffers), which are allocated to normal vertex buffers and whose layout is
/// defined by `vertex_buffer_index`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct GraphicsPipeline {
    data: RefEqArc<GraphicsPipelineData>,
}

#[derive(Debug)]
struct GraphicsPipelineData {
    metal_pipeline: OCPtr<metal::MTLRenderPipelineState>,

    /// Defines a mapping from vertex buffer binding indices to Metal vertex shader's
    /// buffer argument table indices (Metal vertex buffer indices).
    vertex_buffer_map: Vec<Option<usize>>,

    /// To be specified via draw calls
    primitive_type: metal::MTLPrimitiveType,

    /// To be specified via `MTLRenderCommandEncoder`
    raster_data: Option<GraphicsPipelineRasterizerData>,
}

/// Contains pipeline parameters set via
/// a render command encoder and some other parameters
/// related to the rasterizer.
///
/// Members of the type `Option<T>` are `Some` if
/// the values are static and `None` if they are dynamic
/// and are set via a command buffer's methods.
#[derive(Debug)]
struct GraphicsPipelineRasterizerData {
    viewport: Option<metal::MTLViewport>,
    scissor_rect: Option<metal::MTLScissorRect>,
    cull_mode: metal::MTLCullMode,
    front_face: metal::MTLWinding,
    depth_clip_mode: metal::MTLDepthClipMode,
    triangle_fill_mode: metal::MTLTriangleFillMode,
    metal_ds_state: Option<(OCPtr<metal::MTLDepthStencilState>, StencilReference)>,
    blend_constants: Option<[f32; 4]>,

    /// Used during creating a `MTLDepthStencilDescriptor`
    depth_write: bool,
    /// Used during creating a `MTLDepthStencilDescriptor`
    depth_test: metal::MTLCompareFunction,
}

#[derive(Debug, Clone)]
struct StencilReference {
    pub front: u32,
    pub back: u32,
}

unsafe impl Send for GraphicsPipelineData {}
unsafe impl Sync for GraphicsPipelineData {} // no interior mutability

impl core::Marker for GraphicsPipeline {
    fn set_label(&self, label: Option<&str>) {
        // TODO: [MTLRenderPipelineState setLabel:] fails with the following error:
        //       `[MTLIGRenderPipelineState setLabel:]: unrecognized selector sent to instance ...`
        /* self.data.metal_pipeline.set_label(label.unwrap_or(""));
        if let Some(GraphicsPipelineRasterizerData {
                        metal_ds_state: Some((ref state, _)), ..
                    }) = self.data.raster_data
        {
            state.set_label(label.unwrap_or(""));
        } */
    }
}

impl core::GraphicsPipeline for GraphicsPipeline {}

impl GraphicsPipeline {
    pub(crate) fn new(
        metal_device: metal::MTLDevice,
        desc: &imp::GraphicsPipelineDescription,
    ) -> core::Result<GraphicsPipeline> {
        assert!(!metal_device.is_null());
        let metal_desc =
            unsafe { OCPtr::from_raw(metal::MTLRenderPipelineDescriptor::alloc().init()).unwrap() };

        // TODO: GraphicsPipelineDescription validation

        // Collect shaders
        let mut vertex_stage: Option<&core::ShaderStageDescription<imp::ShaderModule>> = None;
        let mut fragment_stage: Option<&core::ShaderStageDescription<imp::ShaderModule>> = None;
        for stage in desc.shader_stages.iter() {
            match stage.stage {
                core::ShaderStageFlags::Fragment => {
                    assert!(fragment_stage.is_none(), "duplicate fragment shader stages");
                    fragment_stage = Some(stage);
                }
                core::ShaderStageFlags::Vertex => {
                    assert!(vertex_stage.is_none(), "duplicate vertex shader stages");
                    vertex_stage = Some(stage);
                }
                core::ShaderStageFlags::Compute => {
                    panic!("compute shader stage cannot be specified in a graphics pipeline");
                }
            }
        }

        // Allocate vertex buffers (skip vertex buffers that are not actually used)
        // And scan through all vertex attributes (create map between VA and Metal VS BAT)
        let start_metal_vb_index = desc.pipeline_layout.num_vertex_shader_buffers();
        let mut next_metal_vb_index = start_metal_vb_index;
        let vertex_buffer_map_size = desc.vertex_buffers
            .iter()
            .map(|vbl| vbl.binding)
            .max()
            .map(|count| count + 1)
            .unwrap_or(0);
        let mut vertex_buffer_map = vec![None; vertex_buffer_map_size];
        let mut vertex_attrs = Vec::with_capacity(desc.vertex_attributes.len());
        for attr in desc.vertex_attributes {
            let ref mut vertex_buffer_index_ref = vertex_buffer_map[attr.binding];
            if vertex_buffer_index_ref.is_none() {
                // Allocate a new vertex shader buffer argument table entry
                *vertex_buffer_index_ref = Some(next_metal_vb_index);
                next_metal_vb_index += 1;
            }
            let vertex_buffer_index = vertex_buffer_index_ref.unwrap();
            vertex_attrs.push((attr, vertex_buffer_index));
        }

        // Setup MTLVertexDescriptor
        let metal_vertex_desc = metal_desc.vertex_descriptor();
        assert!(!metal_vertex_desc.is_null());

        let metal_va_array = metal_vertex_desc.attributes();
        for &(attr, vertex_buffer_index) in vertex_attrs.iter() {
            // Vertex attribute locations are directly mapped to
            // Metal vertex attribute indices.
            let metal_va_desc = metal_va_array.object_at(attr.location);
            metal_va_desc.set_buffer_index(vertex_buffer_index as u64);
            metal_va_desc.set_offset(attr.offset as u64);
            metal_va_desc.set_format(imp::translate_vertex_format(attr.format).expect(
                "unsupported vertex format",
            ));
        }

        let metal_vbl_array = metal_vertex_desc.layouts();
        for (gfx_vb_index, metal_vb_index) in vertex_buffer_map.iter().enumerate() {
            if let &Some(metal_vb_index) = metal_vb_index {
                let metal_vbl_desc: metal::MTLVertexBufferLayoutDescriptor =
                    metal_vbl_array.object_at(metal_vb_index);
                let gfx_vb: &core::VertexBufferLayoutDescription = &desc.vertex_buffers
                    [gfx_vb_index];
                let step_fn = match gfx_vb.input_rate {
                    core::VertexInputRate::Instance => metal::MTLVertexStepFunction::PerInstance,
                    core::VertexInputRate::Vertex => metal::MTLVertexStepFunction::PerVertex,
                };
                metal_vbl_desc.set_step_function(step_fn);
                metal_vbl_desc.set_stride(gfx_vb.stride as u64);
            }
        }

        // Create shaders
        let vertex_stage = vertex_stage.expect("missing vertex shader stage");
        let shader_vertex_infos = vertex_attrs.iter().map(|&(attr, metal_vb_index)| {
            let gfx_vb: &core::VertexBufferLayoutDescription = &desc.vertex_buffers[attr.binding];
            imp::ShaderVertexAttributeInfo {
                binding: attr.location,
                msl_buffer_index: metal_vb_index,
                offset: attr.offset,
                stride: gfx_vb.stride,
                input_rate: gfx_vb.input_rate,
            }
        });
        let vertex_fn = vertex_stage.module.get_function(
            vertex_stage.entry_point_name,
            core::ShaderStageFlags::Vertex,
            desc.pipeline_layout,
            metal_device,
            shader_vertex_infos,
        );
        metal_desc.set_vertex_function(*vertex_fn);

        if let Some(fragment_stage) = fragment_stage {
            let fragment_fn = fragment_stage.module.get_function(
                fragment_stage.entry_point_name,
                core::ShaderStageFlags::Fragment,
                desc.pipeline_layout,
                metal_device,
                empty(),
            );
            metal_desc.set_fragment_function(*fragment_fn);
        }

        let prim_type = match desc.topology {
            core::PrimitiveTopology::Points => metal::MTLPrimitiveType::Point,
            core::PrimitiveTopology::Lines => metal::MTLPrimitiveType::Line,
            core::PrimitiveTopology::LineStrip => metal::MTLPrimitiveType::LineStrip,
            core::PrimitiveTopology::Triangles => metal::MTLPrimitiveType::Triangle,
            core::PrimitiveTopology::TriangleStrip => metal::MTLPrimitiveType::TriangleStrip,
        };

        let topo_class = match desc.topology {
            core::PrimitiveTopology::Points => metal::MTLPrimitiveTopologyClass::Point,
            core::PrimitiveTopology::Lines |
            core::PrimitiveTopology::LineStrip => metal::MTLPrimitiveTopologyClass::Line,
            core::PrimitiveTopology::Triangles |
            core::PrimitiveTopology::TriangleStrip => metal::MTLPrimitiveTopologyClass::Triangle,
        };
        metal_desc.set_input_primitive_topology(topo_class);

        let mut raster_data = None;

        if let Some(ref rst) = desc.rasterizer {
            metal_desc.set_rasterization_enabled(true);

            let mut viewport = None;
            if let core::StaticOrDynamic::Static(ref value) = rst.viewport {
                viewport = Some(utils::translate_viewport(value));
            }
            let mut scissor_rect = None;
            if let core::StaticOrDynamic::Static(ref value) = rst.scissor_rect {
                scissor_rect = Some(utils::translate_scissor_rect(value));
            }
            let cull_mode = match rst.cull_mode {
                core::CullMode::None => metal::MTLCullMode::None,
                core::CullMode::Back => metal::MTLCullMode::Back,
                core::CullMode::Front => metal::MTLCullMode::Front,
            };
            let front_face = match rst.front_face {
                core::Winding::Clockwise => metal::MTLWinding::Clockwise,
                core::Winding::CounterClockwise => metal::MTLWinding::CounterClockwise,
            };
            let depth_clip_mode = match rst.depth_clip_mode {
                core::DepthClipMode::Clip => metal::MTLDepthClipMode::Clip,
                core::DepthClipMode::Clamp => metal::MTLDepthClipMode::Clamp,
            };
            let triangle_fill_mode = match rst.triangle_fill_mode {
                core::TriangleFillMode::Line => metal::MTLTriangleFillMode::Lines,
                core::TriangleFillMode::Fill => metal::MTLTriangleFillMode::Fill,
            };

            metal_desc.set_alpha_to_coverage_enabled(rst.alpha_to_coverage);
            metal_desc.set_sample_count(rst.sample_count as u64);

            let depth_write = rst.depth_write;
            let depth_test = utils::translate_compare_function(rst.depth_test);

            let mut metal_ds_state = None;
            if let core::StaticOrDynamic::Static(ref value) = rst.stencil {
                metal_ds_state = Some(make_depth_stencil_state(
                    metal_device,
                    depth_write,
                    depth_test,
                    value,
                    desc.label,
                ));
            }

            let mut blend_constants = None;
            if let core::StaticOrDynamic::Static(ref value) = rst.blend_constants {
                blend_constants = Some(*value);
            }

            raster_data = Some(GraphicsPipelineRasterizerData {
                viewport,
                scissor_rect,
                cull_mode,
                front_face,
                depth_clip_mode,
                triangle_fill_mode,
                depth_write,
                depth_test,
                metal_ds_state,
                blend_constants,
            });

            let render_pass: &imp::RenderPass = desc.render_pass;
            let subpass = desc.subpass_index;
            let color_atts = rst.color_attachments;

            assert_eq!(
                color_atts.len(),
                render_pass.num_subpass_color_attachments(subpass),
                "invalid element count of rasterizer.color_attachments"
            );

            metal_desc.set_depth_attachment_pixel_format(
                render_pass
                    .subpass_depth_attachment_format(subpass)
                    .map(|f| {
                        formats::translate_image_format(f).expect("unsupported image format")
                    })
                    .unwrap_or(metal::MTLPixelFormat::Invalid),
            );

            metal_desc.set_stencil_attachment_pixel_format(
                render_pass
                    .subpass_stencil_attachment_format(subpass)
                    .map(|f| {
                        formats::translate_image_format(f).expect("unsupported image format")
                    })
                    .unwrap_or(metal::MTLPixelFormat::Invalid),
            );

            let metal_color_att_array = metal_desc.color_attachments();
            for (i, color_att) in color_atts.iter().enumerate() {
                // color_att: GraphicsPipelineColorAttachmentDescription
                let metal_color_att: metal::MTLRenderPipelineColorAttachmentDescriptor
                    = metal_color_att_array.object_at(i);

                let format = render_pass.subpass_color_attachment_format(subpass, i);
                let format = format.unwrap(); // FIXME: does Metal support nullifying color attachment access?

                metal_color_att.set_pixel_format(formats::translate_image_format(format).expect(
                    "unsupported image format",
                ));

                if let Some(ref blend_desc) = color_att.blending {
                    metal_color_att.set_blending_enabled(true);
                    metal_color_att.set_source_rgb_blend_factor(
                        translate_blend_factor(blend_desc.source_rgb_factor),
                    );
                    metal_color_att.set_source_alpha_blend_factor(
                        translate_blend_factor(blend_desc.source_alpha_factor),
                    );
                    metal_color_att.set_destination_rgb_blend_factor(
                        translate_blend_factor(blend_desc.destination_rgb_factor),
                    );
                    metal_color_att.set_destination_alpha_blend_factor(
                        translate_blend_factor(blend_desc.destination_alpha_factor),
                    );
                    metal_color_att.set_rgb_blend_operation(translate_blend_op(
                        blend_desc.rgb_blend_operation,
                    ));
                    metal_color_att.set_alpha_blend_operation(
                        translate_blend_op(blend_desc.alpha_blend_operation),
                    );
                } else {
                    metal_color_att.set_blending_enabled(false);
                }

                let mut mask = metal::MTLColorWriteMaskNone;
                if !(color_att.write_mask & core::ColorWriteMask::Red).is_empty() {
                    mask |= metal::MTLColorWriteMaskRed;
                }
                if !(color_att.write_mask & core::ColorWriteMask::Green).is_empty() {
                    mask |= metal::MTLColorWriteMaskGreen;
                }
                if !(color_att.write_mask & core::ColorWriteMask::Blue).is_empty() {
                    mask |= metal::MTLColorWriteMaskBlue;
                }
                if !(color_att.write_mask & core::ColorWriteMask::Alpha).is_empty() {
                    mask |= metal::MTLColorWriteMaskAlpha;
                }
                metal_color_att.set_write_mask(mask);
            }

        // `rst.depth_bounds` is ignored - unsupported (see `limits.rs`)
        } else {
            metal_desc.set_rasterization_enabled(false);
        }

        // `alphaToOneEnabled` is not supported for now

        // set debug label on `MTLRenderPipelineDescriptor`
        if let Some(label) = desc.label {
            metal_desc.set_label(label);
        }

        let metal_pipeline = metal_device
            .new_render_pipeline_state(*metal_desc)
            .map(|p| OCPtr::new(p).unwrap())
            .expect("render pipeline state creation failed");

        let data = GraphicsPipelineData {
            metal_pipeline,
            vertex_buffer_map,
            primitive_type: prim_type,
            raster_data,
        };

        Ok(GraphicsPipeline { data: RefEqArc::new(data) })
    }

    pub(crate) fn bind_pipeline_state(&self, encoder: metal::MTLRenderCommandEncoder) {
        encoder.set_render_pipeline_state(*self.data.metal_pipeline);

        if let Some(GraphicsPipelineRasterizerData {
                        metal_ds_state: Some((ref state, ref s_ref)), ..
                    }) = self.data.raster_data
        {
            encoder.set_depth_stencil_state(**state);
            encoder.set_stencil_front_back_reference_value(s_ref.front, s_ref.back);
        }

        // TODO: set static states
    }

    pub(crate) fn primitive_type(&self) -> metal::MTLPrimitiveType {
        self.data.primitive_type
    }

    pub(crate) fn bind_vertex_buffers(
        &self,
        encoder: metal::MTLRenderCommandEncoder,
        start_index: usize,
        buffers: &[(&imp::Buffer, usize)],
    ) {
        let ref mappings = self.data.vertex_buffer_map;
        for (i, &(buffer, offset)) in buffers.iter().enumerate() {
            let map_index = i + start_index;
            if map_index >= mappings.len() {
                break;
            }

            if let Some(mtl_buffer_idx) = mappings[map_index] {
                encoder.set_vertex_buffer(
                    mtl_buffer_idx as u64,
                    offset as u64,
                    buffer.metal_buffer(),
                );
            }
        }
    }
}

fn translate_blend_factor(value: core::BlendFactor) -> metal::MTLBlendFactor {
    match value {
        core::BlendFactor::Zero => metal::MTLBlendFactor::Zero,
        core::BlendFactor::One => metal::MTLBlendFactor::One,
        core::BlendFactor::SourceColor => metal::MTLBlendFactor::SourceColor,
        core::BlendFactor::OneMinusSourceColor => metal::MTLBlendFactor::OneMinusSourceColor,
        core::BlendFactor::SourceAlpha => metal::MTLBlendFactor::SourceAlpha,
        core::BlendFactor::OneMinusSourceAlpha => metal::MTLBlendFactor::OneMinusSourceAlpha,
        core::BlendFactor::DestinationColor => metal::MTLBlendFactor::DestinationColor,
        core::BlendFactor::OneMinusDestinationColor => {
            metal::MTLBlendFactor::OneMinusDestinationColor
        }
        core::BlendFactor::DestinationAlpha => metal::MTLBlendFactor::DestinationAlpha,
        core::BlendFactor::OneMinusDestinationAlpha => {
            metal::MTLBlendFactor::OneMinusDestinationAlpha
        }
        core::BlendFactor::ConstantColor => metal::MTLBlendFactor::BlendColor,
        core::BlendFactor::OneMinusConstantColor => metal::MTLBlendFactor::OneMinusBlendColor,
        core::BlendFactor::ConstantAlpha => metal::MTLBlendFactor::BlendAlpha,
        core::BlendFactor::OneMinusConstantAlpha => metal::MTLBlendFactor::OneMinusBlendAlpha,
        core::BlendFactor::SourceAlphaSaturated => metal::MTLBlendFactor::SourceAlphaSaturated,
        core::BlendFactor::Source1Color => metal::MTLBlendFactor::Source1Color,
        core::BlendFactor::OneMinusSource1Color => metal::MTLBlendFactor::OneMinusSource1Color,
        core::BlendFactor::Source1Alpha => metal::MTLBlendFactor::Source1Alpha,
        core::BlendFactor::OneMinusSource1Alpha => metal::MTLBlendFactor::OneMinusSource1Alpha,
    }
}

fn translate_blend_op(value: core::BlendOperation) -> metal::MTLBlendOperation {
    match value {
        core::BlendOperation::Add => metal::MTLBlendOperation::Add,
        core::BlendOperation::Subtract => metal::MTLBlendOperation::Subtract,
        core::BlendOperation::ReverseSubtract => metal::MTLBlendOperation::ReverseSubtract,
        core::BlendOperation::Min => metal::MTLBlendOperation::Min,
        core::BlendOperation::Max => metal::MTLBlendOperation::Max,
    }
}

fn translate_stencil_op(value: core::StencilOperation) -> metal::MTLStencilOperation {
    match value {
        core::StencilOperation::Keep => metal::MTLStencilOperation::Keep,
        core::StencilOperation::Zero => metal::MTLStencilOperation::Zero,
        core::StencilOperation::Replace => metal::MTLStencilOperation::Replace,
        core::StencilOperation::IncrementAndClamp => metal::MTLStencilOperation::IncrementClamp,
        core::StencilOperation::DecrementAndClamp => metal::MTLStencilOperation::DecrementClamp,
        core::StencilOperation::Invert => metal::MTLStencilOperation::Invert,
        core::StencilOperation::IncrementAndWrap => metal::MTLStencilOperation::IncrementWrap,
        core::StencilOperation::DecrementAndWrap => metal::MTLStencilOperation::DecrementWrap,
    }
}

fn make_depth_stencil_state(
    metal_device: metal::MTLDevice,
    depth_write: bool,
    depth_test: metal::MTLCompareFunction,
    stencil_state: &core::StencilDescriptionSet,
    label: Option<&str>,
) -> (OCPtr<metal::MTLDepthStencilState>, StencilReference) {
    let metal_desc =
        unsafe { OCPtr::from_raw(metal::MTLDepthStencilDescriptor::alloc().init()).unwrap() };

    metal_desc.set_depth_write_enabled(depth_write);
    metal_desc.set_depth_compare_function(depth_test);

    for &(mtl_stencil, gfx_stencil) in
        [
            (metal_desc.front_face_stencil(), &stencil_state.front),
            (metal_desc.back_face_stencil(), &stencil_state.back),
        ].iter()
    {
        mtl_stencil.set_stencil_compare_function(
            translate_compare_function(
                gfx_stencil.compare_function,
            ),
        );
        mtl_stencil.set_stencil_failure_operation(
            translate_stencil_op(
                gfx_stencil.stencil_fail_operation,
            ),
        );
        mtl_stencil.set_depth_failure_operation(
            translate_stencil_op(
                gfx_stencil.depth_fail_operation,
            ),
        );
        mtl_stencil.set_depth_stencil_pass_operation(
            translate_stencil_op(gfx_stencil.pass_operation),
        );
        mtl_stencil.set_read_mask(gfx_stencil.read_mask);
        mtl_stencil.set_write_mask(gfx_stencil.write_mask);
    }

    let ref_value = StencilReference {
        front: stencil_state.front.reference,
        back: stencil_state.back.reference,
    };

    if let Some(label) = label {
        metal_desc.set_label(label);
    }

    (
        OCPtr::new(metal_device.new_depth_stencil_state(*metal_desc)).unwrap(),
        ref_value,
    )
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ComputePipeline {
    data: RefEqArc<ComputePipelineData>,
}

#[derive(Debug)]
struct ComputePipelineData {
    // TODO: metal_pipeline: OCPtr<metal::MTLComputePipelineState>,
}

unsafe impl Send for ComputePipelineData {}
unsafe impl Sync for ComputePipelineData {} // no interior mutability

impl core::Marker for ComputePipeline {
    fn set_label(&self, _: Option<&str>) {
        unimplemented!()
        // self.data.metal_pipeline.set_label(label.unwrap_or(""));
    }
}

impl core::ComputePipeline for ComputePipeline {
    fn max_num_workgroup_invocations(&self) -> u32 {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StencilState {
    data: RefEqArc<StencilStateData>,
}

#[derive(Debug)]
struct StencilStateData {
    metal_ds_state: OCPtr<metal::MTLDepthStencilState>,
    stencil_ref: StencilReference,
}

unsafe impl Send for StencilStateData {}
unsafe impl Sync for StencilStateData {} // no interior mutability

impl core::Marker for StencilState {
    fn set_label(&self, label: Option<&str>) {
        self.data.metal_ds_state.set_label(label.unwrap_or(""));
    }
}

impl core::StencilState for StencilState {}

impl StencilState {
    pub(crate) fn new(
        metal_device: metal::MTLDevice,
        desc: &imp::StencilStateDescription,
    ) -> core::Result<StencilState> {
        let raster_data = desc.pipeline.data.raster_data.as_ref().expect(
            "graphics pipeline does not have a rasterizer enabled",
        );
        let (metal_ds_state, stencil_ref) = make_depth_stencil_state(
            metal_device,
            raster_data.depth_write,
            raster_data.depth_test,
            &desc.set,
            desc.label,
        );

        let data = StencilStateData {
            metal_ds_state,
            stencil_ref,
        };

        Ok(StencilState { data: RefEqArc::new(data) })
    }

    pub(crate) fn bind_depth_stencil_state(&self, encoder: metal::MTLRenderCommandEncoder) {
        encoder.set_depth_stencil_state(*self.data.metal_ds_state);

        let ref s_ref = self.data.stencil_ref;
        encoder.set_stencil_front_back_reference_value(s_ref.front, s_ref.back);
    }
}
