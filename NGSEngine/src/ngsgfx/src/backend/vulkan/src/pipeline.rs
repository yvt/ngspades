//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::vk;
use ash::version::DeviceV1_0;
use std::{ptr, ffi};

use {RefEqArc, DeviceRef, AshDevice, translate_generic_error_unwrap};
use imp::{self, ShaderModule, translate_shader_stage, translate_vertex_format,
          translate_rect2d_u32, translate_compare_function, LlFence};
use command::mutex::{ResourceMutex, ResourceMutexDeviceRef};

pub struct GraphicsPipeline<T: DeviceRef> {
    data: RefEqArc<GraphicsPipelineData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for GraphicsPipeline<T> => data
}

#[derive(Debug)]
struct GraphicsPipelineData<T: DeviceRef> {
    mutex: ResourceMutex<LlFence<T>, GraphicsPipelineLockData<T>>,
}

#[derive(Debug)]
pub(crate) struct GraphicsPipelineLockData<T: DeviceRef> {
    device_ref: T,
    handle: vk::Pipeline,
}

impl<T: DeviceRef> core::GraphicsPipeline for GraphicsPipeline<T> {}

impl<T: DeviceRef> Drop for GraphicsPipelineLockData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.destroy_pipeline(self.handle, self.device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> GraphicsPipeline<T> {
    pub(crate) fn new(
        device_ref: &T,
        desc: &imp::GraphicsPipelineDescription<T>,
    ) -> core::Result<Self> {
        use std::ffi::CString;
        use core::StaticOrDynamic::{Static, Dynamic};

        let mut dyn_states = Vec::with_capacity(9);

        let stage_names: Vec<_> = desc.shader_stages
            .iter()
            .map(|ssd| CString::new(ssd.entry_point_name).unwrap())
            .collect();
        let stages: Vec<_> = desc.shader_stages
            .iter()
            .zip(stage_names.iter())
            .map(|(ssd, name)| vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PipelineShaderStageCreateInfo,
                p_next: ptr::null(),
                flags: vk::PipelineShaderStageCreateFlags::empty(),
                stage: translate_shader_stage(ssd.stage),
                module: ssd.module.handle(),
                p_name: name.as_ptr(),
                p_specialization_info: ptr::null(),
            })
            .collect();

        let vertex_bindings: Vec<_> = desc.vertex_buffers
            .iter()
            .map(|vbld| vk::VertexInputBindingDescription {
                binding: vbld.binding as u32,
                stride: vbld.stride,
                input_rate: match vbld.input_rate {
                    core::VertexInputRate::Vertex => vk::VertexInputRate::Vertex,
                    core::VertexInputRate::Instance => vk::VertexInputRate::Instance,
                },
            })
            .collect();
        let vertex_attrs: Vec<_> = desc.vertex_attributes
            .iter()
            .map(|vad| vk::VertexInputAttributeDescription {
                location: vad.location as u32,
                binding: vad.binding as u32,
                format: translate_vertex_format(vad.format)
                    .expect("unsupported vertex format"),
                offset: vad.offset,
            })
            .collect();
        let vis_info = vk::PipelineVertexInputStateCreateInfo{
            s_type: vk::StructureType::PipelineVertexInputStateCreateInfo,
            p_next: ptr::null(),
            flags: vk::PipelineVertexInputStateCreateFlags::empty(),
            vertex_binding_description_count: vertex_bindings.len() as u32,
            p_vertex_binding_descriptions: vertex_bindings.as_ptr(),
            vertex_attribute_description_count: vertex_attrs.len() as u32,
            p_vertex_attribute_descriptions: vertex_attrs.as_ptr(),
        };

        let ias_info = vk::PipelineInputAssemblyStateCreateInfo{
            s_type: vk::StructureType::PipelineInputAssemblyStateCreateInfo,
            p_next: ptr::null(),
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            topology: match desc.topology {
                core::PrimitiveTopology::Points => vk::PrimitiveTopology::PointList,
                core::PrimitiveTopology::Lines => vk::PrimitiveTopology::LineList,
                core::PrimitiveTopology::LineStrip => vk::PrimitiveTopology::LineStrip,
                core::PrimitiveTopology::Triangles => vk::PrimitiveTopology::TriangleList,
                core::PrimitiveTopology::TriangleStrip => vk::PrimitiveTopology::TriangleStrip,
            },
            primitive_restart_enable: vk::VK_TRUE,
        };

        let mut viewports = [vk::Viewport{
            x: 0f32, y: 0f32, width: 0f32, height: 0f32, min_depth: 0f32, max_depth: 1f32,
        }];
        let mut scissors = [vk::Rect2D{
            offset: vk::Offset2D{ x: 0, y: 0 },
            extent: vk::Extent2D{ width: 0, height: 0 },
        }];
        let mut cb_attachments = Vec::new();

        let r_infos = if let Some(ref rd) = desc.rasterizer {
            let rs_info = vk::PipelineRasterizationStateCreateInfo{
                s_type: vk::StructureType::PipelineRasterizationStateCreateInfo,
                p_next: ptr::null(),
                flags: vk::PipelineRasterizationStateCreateFlags::empty(),
                depth_clamp_enable: match rd.depth_clip_mode {
                    core::DepthClipMode::Clip => vk::VK_FALSE,
                    core::DepthClipMode::Clamp => vk::VK_TRUE,
                },
                rasterizer_discard_enable: vk::VK_FALSE,
                polygon_mode: match rd.triangle_fill_mode {
                    core::TriangleFillMode::Fill => vk::PolygonMode::Fill,
                    core::TriangleFillMode::Line => vk::PolygonMode::Line,
                },
                cull_mode: match rd.cull_mode {
                    core::CullMode::None => vk::CULL_MODE_NONE,
                    core::CullMode::Front => vk::CULL_MODE_FRONT_BIT,
                    core::CullMode::Back => vk::CULL_MODE_BACK_BIT,
                },
                front_face: match rd.front_face {
                    core::Winding::Clockwise => vk::FrontFace::Clockwise,
                    core::Winding::CounterClockwise => vk::FrontFace::CounterClockwise,
                },
                depth_bias_enable: match rd.depth_bias {
                    Dynamic | Static(Some(_)) => vk::VK_TRUE,
                    Static(None) => vk::VK_FALSE,
                },
                depth_bias_constant_factor: match rd.depth_bias {
                    Static(Some(ref db)) => db.constant_factor,
                    _ => 0f32,
                },
                depth_bias_clamp: match rd.depth_bias {
                    Static(Some(ref db)) => db.clamp,
                    _ => 0f32,
                },
                depth_bias_slope_factor: match rd.depth_bias {
                    Static(Some(ref db)) => db.slope_factor,
                    _ => 0f32,
                },
                line_width: 1f32,
            };
            if rd.depth_bias.is_dynamic() {
                dyn_states.push(vk::DynamicState::DepthBias);
            }

            let vs_info = vk::PipelineViewportStateCreateInfo{
                s_type: vk::StructureType::PipelineViewportStateCreateInfo,
                p_next: ptr::null(),
                flags: vk::PipelineViewportStateCreateFlags::empty(),
                viewport_count: 1,
                p_viewports: viewports.as_ptr(),
                scissor_count: 1,
                p_scissors: scissors.as_ptr(),
            };
            match rd.viewport {
                Static(ref vp) => {
                    viewports[0] = vk::Viewport {
                        x: vp.x, y: vp.y,
                        width: vp.width, height: vp.height,
                        min_depth: vp.min_depth, max_depth: vp.max_depth,
                    };
                }
                Dynamic => {
                    dyn_states.push(vk::DynamicState::Viewport);
                }
            }
            match rd.scissor_rect {
                Static(ref r) => {
                    scissors[0] = translate_rect2d_u32(r);
                }
                Dynamic => {
                    dyn_states.push(vk::DynamicState::Scissor);
                }
            }

            let ms_info = vk::PipelineMultisampleStateCreateInfo {
                s_type: vk::StructureType::PipelineMultisampleStateCreateInfo,
                p_next: ptr::null(),
                flags: vk::PipelineMultisampleStateCreateFlags::empty(),
                rasterization_samples: vk::SAMPLE_COUNT_1_BIT,
                sample_shading_enable: vk::VK_FALSE,
                min_sample_shading: 1f32,
                p_sample_mask: ptr::null(),
                alpha_to_coverage_enable: if rd.alpha_to_coverage {
                    vk::VK_TRUE
                } else {
                    vk::VK_FALSE
                },
                alpha_to_one_enable: vk::VK_FALSE,
            };

            let stencil_test_enable =
                rd.stencil_ops[0].compare_function != core::CompareFunction::Always ||
                rd.stencil_ops[1].compare_function != core::CompareFunction::Always;
            let translate_so_state = |i: usize| {
                vk::StencilOpState{
                    fail_op: translate_stencil_op(rd.stencil_ops[i].stencil_fail_operation),
                    depth_fail_op: translate_stencil_op(rd.stencil_ops[i].depth_fail_operation),
                    pass_op: translate_stencil_op(rd.stencil_ops[i].pass_operation),
                    compare_op: translate_compare_function(rd.stencil_ops[i].compare_function),
                    compare_mask: match rd.stencil_masks {
                        Static(ref sm) => sm[i].read_mask,
                        _ => 0u32,
                    },
                    write_mask: match rd.stencil_masks {
                        Static(ref sm) => sm[i].write_mask,
                        _ => 0u32,
                    },
                    reference: match rd.stencil_references {
                        Static(ref sm) => sm[i],
                        _ => 0u32,
                    },
                }
            };
            let dss_info = vk::PipelineDepthStencilStateCreateInfo {
                s_type: vk::StructureType::PipelineDepthStencilStateCreateInfo,
                p_next: ptr::null(),
                flags: vk::PipelineDepthStencilStateCreateFlags::empty(),
                depth_test_enable: if rd.depth_test != core::CompareFunction::Always {
                    vk::VK_TRUE
                } else {
                    vk::VK_FALSE
                },
                depth_write_enable: if rd.depth_write {
                    vk::VK_TRUE
                } else {
                    vk::VK_FALSE
                },
                depth_compare_op: translate_compare_function(rd.depth_test),
                depth_bounds_test_enable: if rd.depth_bounds.is_some() {
                    vk::VK_TRUE
                } else {
                    vk::VK_FALSE
                },
                min_depth_bounds: match rd.depth_bounds {
                    Some(Static(ref db)) => db.min,
                    _ => 0f32,
                },
                max_depth_bounds: match rd.depth_bounds {
                    Some(Static(ref db)) => db.max,
                    _ => 0f32,
                },
                stencil_test_enable: if stencil_test_enable {
                    vk::VK_TRUE
                } else {
                    vk::VK_FALSE
                },
                front: translate_so_state(0),
                back: translate_so_state(1),
            };

            if rd.stencil_masks.is_dynamic() {
                dyn_states.push(vk::DynamicState::StencilCompareMask);
                dyn_states.push(vk::DynamicState::StencilWriteMask);
            }
            if rd.stencil_references.is_dynamic() {
                dyn_states.push(vk::DynamicState::StencilReference);
            }
            if let Some(Dynamic) = rd.depth_bounds {
                dyn_states.push(vk::DynamicState::DepthBounds);
            }

            cb_attachments = rd.color_attachments
                .iter()
                .map(|cad| {
                    let blending = cad.blending.unwrap_or_else(Default::default);
                    let mut mask = vk::ColorComponentFlags::empty();

                    if cad.write_mask.contains(core::ColorWriteMask::Red) {
                        mask |= vk::COLOR_COMPONENT_R_BIT;
                    }
                    if cad.write_mask.contains(core::ColorWriteMask::Green) {
                        mask |= vk::COLOR_COMPONENT_G_BIT;
                    }
                    if cad.write_mask.contains(core::ColorWriteMask::Blue) {
                        mask |= vk::COLOR_COMPONENT_B_BIT;
                    }
                    if cad.write_mask.contains(core::ColorWriteMask::Alpha) {
                        mask |= vk::COLOR_COMPONENT_A_BIT;
                    }

                    vk::PipelineColorBlendAttachmentState {
                        blend_enable: if cad.blending.is_some() {
                            vk::VK_TRUE
                        } else {
                            vk::VK_FALSE
                        },
                        src_color_blend_factor: translate_blend_factor(blending.source_rgb_factor),
                        src_alpha_blend_factor: translate_blend_factor(blending.source_alpha_factor),
                        dst_color_blend_factor: translate_blend_factor(blending.destination_rgb_factor),
                        dst_alpha_blend_factor: translate_blend_factor(blending.destination_alpha_factor),
                        color_blend_op: translate_blend_op(blending.rgb_blend_operation),
                        alpha_blend_op: translate_blend_op(blending.alpha_blend_operation),
                        color_write_mask: mask,
                    }
                })
                .collect();

            let cbs_info = vk::PipelineColorBlendStateCreateInfo {
                s_type: vk::StructureType::PipelineColorBlendStateCreateInfo,
                p_next: ptr::null(),
                flags: vk::PipelineColorBlendStateCreateFlags::empty(),
                logic_op_enable: vk::VK_FALSE,
                logic_op: vk::LogicOp::Copy,
                attachment_count: cb_attachments.len() as u32,
                p_attachments: cb_attachments.as_ptr(),
                blend_constants: match rd.blend_constants {
                    Static(values) => values,
                    Dynamic => Default::default(),
                },
            };
            if rd.blend_constants.is_dynamic() {
                dyn_states.push(vk::DynamicState::BlendConstants);
            }

            Some((rs_info, vs_info, ms_info, dss_info, cbs_info))
        } else {
            // Suppress "unused assignment" warning.
            // `cb_attachments` is put onto the top level so that
            // `cb_attachments.as_ptr()` lives long enough
            let _ = cb_attachments.as_ptr();
            None
        };

        let rs_info_none = vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PipelineRasterizationStateCreateInfo,
            p_next: ptr::null(),
            flags: vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: vk::VK_FALSE,
            rasterizer_discard_enable: vk::VK_TRUE,
            polygon_mode: vk::PolygonMode::Fill,
            cull_mode: vk::CULL_MODE_FRONT_AND_BACK,
            front_face: vk::FrontFace::CounterClockwise,
            depth_bias_enable: vk::VK_FALSE,
            depth_bias_constant_factor: 0f32,
            depth_bias_clamp: 0f32,
            depth_bias_slope_factor: 0f32,
            line_width: 1f32,
        };

        let dyn_state = vk::PipelineDynamicStateCreateInfo{
            s_type: vk::StructureType::PipelineDynamicStateCreateInfo,
            p_next: ptr::null(),
            flags: vk::PipelineDynamicStateCreateFlags::empty(),
            dynamic_state_count: dyn_states.len() as u32,
            p_dynamic_states: dyn_states.as_ptr(),
        };

        let info = vk::GraphicsPipelineCreateInfo{
            s_type: vk::StructureType::GraphicsPipelineCreateInfo,
            p_next: ptr::null(),
            flags: vk::PipelineCreateFlags::empty(),
            stage_count: stages.len() as u32,
            p_stages: stages.as_ptr(),
            p_vertex_input_state: &vis_info,
            p_input_assembly_state: &ias_info,
            p_tessellation_state: ptr::null(),
            p_rasterization_state: r_infos.as_ref().map(|x|&x.0 as *const _).unwrap_or(&rs_info_none),
            p_viewport_state: r_infos.as_ref().map(|x|&x.1 as *const _).unwrap_or(ptr::null()),
            p_multisample_state: r_infos.as_ref().map(|x|&x.2 as *const _).unwrap_or(ptr::null()),
            p_depth_stencil_state: r_infos.as_ref().map(|x|&x.3 as *const _).unwrap_or(ptr::null()),
            p_color_blend_state: r_infos.as_ref().map(|x|&x.4 as *const _).unwrap_or(ptr::null()),
            p_dynamic_state: &dyn_state,
            layout: desc.pipeline_layout.handle(),
            render_pass: desc.render_pass.handle(),
            subpass: desc.subpass_index as u32,
            base_pipeline_handle: vk::Pipeline::null(),
            base_pipeline_index: 0,
        };

        let handle;
        {
            let device: &AshDevice = device_ref.device();
            handle = unsafe {
                device.create_graphics_pipelines(
                    vk::PipelineCache::null(), &[info], device_ref.allocation_callbacks()
                )
            }.map_err(|x| translate_generic_error_unwrap(x.1))?[0];
        }

        Ok(GraphicsPipeline {
            data: RefEqArc::new(GraphicsPipelineData {
                mutex: ResourceMutex::new(GraphicsPipelineLockData{
                    device_ref: device_ref.clone(),
                    handle,
                }, false),
            }),
        })
    }

    pub fn handle(&self) -> vk::Pipeline {
        self.data.mutex.get_host_read().handle
    }

    pub(crate) fn lock_device(&self) -> ResourceMutexDeviceRef<LlFence<T>, GraphicsPipelineLockData<T>> {
        self.data.mutex.expect_device_access().0
    }
}

pub struct StencilState {
    data: RefEqArc<StencilStateData>,
}

derive_using_field! {
    (); (PartialEq, Eq, Hash, Debug, Clone) for StencilState => data
}

#[derive(Debug)]
struct StencilStateData {}

impl core::StencilState for StencilState {}

pub struct ComputePipeline<T: DeviceRef> {
    data: RefEqArc<ComputePipelineData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for ComputePipeline<T> => data
}

#[derive(Debug)]
struct ComputePipelineData<T: DeviceRef> {
    mutex: ResourceMutex<LlFence<T>, ComputePipelineLockData<T>>,
}

#[derive(Debug)]
pub(crate) struct ComputePipelineLockData<T: DeviceRef> {
    device_ref: T,
    handle: vk::Pipeline,
}

impl<T: DeviceRef> core::ComputePipeline for ComputePipeline<T> {}

impl<T: DeviceRef> Drop for ComputePipelineLockData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.destroy_pipeline(self.handle, self.device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> ComputePipeline<T> {
    pub(crate) fn new(
        device_ref: &T,
        desc: &imp::ComputePipelineDescription<T>,
    ) -> core::Result<Self> {
        let stage = translate_shader_stage_description(&desc.shader_stage);
        let info = vk::ComputePipelineCreateInfo {
            s_type: vk::StructureType::ComputePipelineCreateInfo,
            p_next: ptr::null(),
            flags: vk::PipelineCreateFlags::empty(),
            stage: stage.0,
            layout: desc.pipeline_layout.handle(),
            base_pipeline_handle: vk::Pipeline::null(),
            base_pipeline_index: -1,
        };

        let device_ref = device_ref.clone();
        let cache = vk::PipelineCache::null();
        let handle;
        {
            let device: &AshDevice = device_ref.device();
            handle = unsafe {
                device.create_compute_pipelines(cache, &[info], device_ref.allocation_callbacks())
            }.map_err(|e| translate_pipeline_creation_error_unwrap(&device_ref, e))?
                [0];
        }

        Ok(ComputePipeline {
            data: RefEqArc::new(ComputePipelineData {
                mutex: ResourceMutex::new(ComputePipelineLockData {
                    device_ref, handle
                }, false),
            }),
        })
    }

    pub fn handle(&self) -> vk::Pipeline {
        self.data.mutex.get_host_read().handle
    }

    pub(crate) fn lock_device(&self) -> ResourceMutexDeviceRef<LlFence<T>, ComputePipelineLockData<T>> {
        self.data.mutex.expect_device_access().0
    }
}

fn translate_pipeline_creation_error_unwrap<T: DeviceRef>(
    device_ref: &T,
    (pipelines, error): (Vec<vk::Pipeline>, vk::Result),
) -> core::GenericError {
    let device: &AshDevice = device_ref.device();

    // First, destroy all successfully created pipelines
    for pl in pipelines {
        if pl != vk::Pipeline::null() {
            unsafe { device.destroy_pipeline(pl, device_ref.allocation_callbacks()) };
        }
    }

    // And then convert the error code
    translate_generic_error_unwrap(error)
}

/// Constructs `vk::PipelineShaderStageCreateInfo` from `core::ShaderStageDescription`.
///
/// Returns a created `vk::PipelineShaderStageCreateInfo` and `CString`.
/// The returned `CString` should live at least as long as the `vk::PipelineShaderStageCreateInfo` is used.
fn translate_shader_stage_description<T: DeviceRef>(
    desc: &core::ShaderStageDescription<ShaderModule<T>>,
) -> (vk::PipelineShaderStageCreateInfo, ffi::CString) {
    let stage = match desc.stage {
        core::ShaderStage::Vertex => vk::SHADER_STAGE_VERTEX_BIT,
        core::ShaderStage::Fragment => vk::SHADER_STAGE_FRAGMENT_BIT,
        core::ShaderStage::Compute => vk::SHADER_STAGE_COMPUTE_BIT,
    };

    let name = ffi::CString::new(desc.entry_point_name).unwrap();

    (
        vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PipelineShaderStageCreateInfo,
            p_next: ptr::null(),
            flags: vk::PipelineShaderStageCreateFlags::empty(), // reserved for future use
            stage,
            module: desc.module.handle(),
            p_name: name.as_ptr(),
            p_specialization_info: ptr::null(),
        },
        name,
    )
}

fn translate_stencil_op(value: core::StencilOperation) -> vk::StencilOp {
    match value {
        core::StencilOperation::Keep => vk::StencilOp::Keep,
        core::StencilOperation::Zero => vk::StencilOp::Zero,
        core::StencilOperation::Replace => vk::StencilOp::Replace,
        core::StencilOperation::IncrementAndClamp => vk::StencilOp::IncrementAndClamp,
        core::StencilOperation::DecrementAndClamp => vk::StencilOp::DecrementAndClamp,
        core::StencilOperation::Invert => vk::StencilOp::Invert,
        core::StencilOperation::IncrementAndWrap => vk::StencilOp::IncrementAndWrap,
        core::StencilOperation::DecrementAndWrap => vk::StencilOp::DecrementAndWrap,
    }
}

fn translate_blend_factor(value: core::BlendFactor) -> vk::BlendFactor {
    match value {
        core::BlendFactor::Zero => vk::BlendFactor::Zero,
        core::BlendFactor::One => vk::BlendFactor::One,
        core::BlendFactor::SourceColor => vk::BlendFactor::SrcColor,
        core::BlendFactor::OneMinusSourceColor => vk::BlendFactor::OneMinusSrcColor,
        core::BlendFactor::SourceAlpha => vk::BlendFactor::SrcAlpha,
        core::BlendFactor::OneMinusSourceAlpha => vk::BlendFactor::OneMinusSrcAlpha,
        core::BlendFactor::DestinationColor => vk::BlendFactor::DstColor,
        core::BlendFactor::OneMinusDestinationColor => vk::BlendFactor::OneMinusDstColor,
        core::BlendFactor::DestinationAlpha => vk::BlendFactor::DstAlpha,
        core::BlendFactor::OneMinusDestinationAlpha => vk::BlendFactor::OneMinusDstAlpha,
        core::BlendFactor::ConstantColor => vk::BlendFactor::ConstantColor,
        core::BlendFactor::OneMinusConstantColor => vk::BlendFactor::OneMinusConstantColor,
        core::BlendFactor::ConstantAlpha => vk::BlendFactor::ConstantAlpha,
        core::BlendFactor::OneMinusConstantAlpha => vk::BlendFactor::OneMinusConstantAlpha,
        core::BlendFactor::SourceAlphaSaturated => vk::BlendFactor::SrcAlphaSaturate,
        core::BlendFactor::Source1Color => vk::BlendFactor::Src1Color,
        core::BlendFactor::OneMinusSource1Color => vk::BlendFactor::OneMinusSrc1Color,
        core::BlendFactor::Source1Alpha => vk::BlendFactor::Src1Alpha,
        core::BlendFactor::OneMinusSource1Alpha => vk::BlendFactor::OneMinusSrc1Alpha,
    }
}

fn translate_blend_op(value: core::BlendOperation) -> vk::BlendOp {
    match value {
        core::BlendOperation::Add => vk::BlendOp::Add,
        core::BlendOperation::Subtract => vk::BlendOp::Subtract,
        core::BlendOperation::ReverseSubtract => vk::BlendOp::ReverseSubtract,
        core::BlendOperation::Min => vk::BlendOp::Min,
        core::BlendOperation::Max => vk::BlendOp::Max,
    }
}