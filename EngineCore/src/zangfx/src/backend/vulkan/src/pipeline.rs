//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of pipelines for Vulkan.
use ash::version::*;
use ash::vk;
use refeq::RefEqArc;
use std::ffi;
use std::ops::Range;

use zangfx_base as base;
use zangfx_base::StaticOrDynamic::*;
use zangfx_base::{zangfx_impl_handle, zangfx_impl_object};
use zangfx_base::{Error, Rect2D, Result};

use crate::arg::layout::RootSig;
use crate::device::DeviceRef;
use crate::formats::translate_vertex_format;
use crate::renderpass::RenderPass;
use crate::shader::Library;
use crate::utils::{
    clip_rect2d_u31, translate_bool, translate_color_channel_flags, translate_compare_op,
    translate_generic_error_unwrap, translate_rect2d_u32, translate_sample_count,
    translate_shader_stage,
};

/// Constructs `vk::PipelineShaderStageCreateInfo`.
///
/// Returns a created `vk::PipelineShaderStageCreateInfo` and `CString`.
/// The returned `CString` should live at least as long as the
/// `vk::PipelineShaderStageCreateInfo` is used.
fn new_shader_stage_description(
    stage: base::ShaderStage,
    library: &Library,
    entry_point_name: &str,
) -> (vk::PipelineShaderStageCreateInfo, ffi::CString) {
    let stage = translate_shader_stage(stage);

    let name = ffi::CString::new(entry_point_name).unwrap();

    (
        vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PipelineShaderStageCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineShaderStageCreateFlags::empty(), // reserved for future use
            stage,
            module: library.vk_shader_module(),
            p_name: name.as_ptr(),
            p_specialization_info: crate::null(),
        },
        name,
    )
}

fn translate_pipeline_creation_error_unwrap(
    device: &DeviceRef,
    (pipelines, error): (Vec<vk::Pipeline>, vk::Result),
) -> Error {
    let device = device.vk_device();

    // First, destroy all successfully created pipelines
    for pl in pipelines {
        if pl != vk::Pipeline::null() {
            unsafe { device.destroy_pipeline(pl, None) };
        }
    }

    // And then convert the error code
    translate_generic_error_unwrap(error)
}

/// Implementation of `ComputePipelineBuilder` for Vulkan.
#[derive(Debug)]
pub struct ComputePipelineBuilder {
    device: DeviceRef,
    compute_shader: Option<(Library, String)>,
    root_sig: Option<RootSig>,
}

zangfx_impl_object! { ComputePipelineBuilder: dyn base::ComputePipelineBuilder, dyn (crate::Debug) }

impl ComputePipelineBuilder {
    crate fn new(device: DeviceRef) -> Self {
        Self {
            device,
            compute_shader: None,
            root_sig: None,
        }
    }
}

impl base::ComputePipelineBuilder for ComputePipelineBuilder {
    fn compute_shader(
        &mut self,
        library: &base::LibraryRef,
        entry_point: &str,
    ) -> &mut dyn base::ComputePipelineBuilder {
        let my_library: &Library = library.downcast_ref().expect("bad library type");
        self.compute_shader = Some((my_library.clone(), entry_point.to_owned()));
        self
    }

    fn root_sig(&mut self, v: &base::RootSigRef) -> &mut dyn base::ComputePipelineBuilder {
        let my_root_sig: &RootSig = v.downcast_ref().expect("bad root signature type");
        self.root_sig = Some(my_root_sig.clone());
        self
    }

    fn build(&mut self) -> Result<base::ComputePipelineRef> {
        let compute_shader = self.compute_shader.as_ref().expect("compute_shader");
        let root_sig = self.root_sig.as_ref().expect("root_sig");

        let stage = new_shader_stage_description(
            base::ShaderStage::Compute,
            &compute_shader.0,
            &compute_shader.1,
        );

        let info = vk::ComputePipelineCreateInfo {
            s_type: vk::StructureType::ComputePipelineCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineCreateFlags::empty(),
            stage: stage.0,
            layout: root_sig.vk_pipeline_layout(),
            base_pipeline_handle: vk::Pipeline::null(),
            base_pipeline_index: -1,
        };

        let cache = vk::PipelineCache::null();

        let vk_device = self.device.vk_device();
        let vk_pipeline = unsafe { vk_device.create_compute_pipelines(cache, &[info], None) }
            .map_err(|e| translate_pipeline_creation_error_unwrap(&self.device, e))?[0];

        Ok(
            unsafe {
                ComputePipeline::from_raw(self.device.clone(), vk_pipeline, root_sig.clone())
            }.into(),
        )
    }
}

/// Implementation of `ComputePipeline` for Vulkan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComputePipeline {
    data: RefEqArc<ComputePipelineData>,
}

zangfx_impl_handle! { ComputePipeline, base::ComputePipelineRef }

#[derive(Debug)]
struct ComputePipelineData {
    device: DeviceRef,
    vk_pipeline: vk::Pipeline,
    root_sig: RootSig,
}

impl ComputePipeline {
    pub(crate) unsafe fn from_raw(
        device: DeviceRef,
        vk_pipeline: vk::Pipeline,
        root_sig: RootSig,
    ) -> Self {
        Self {
            data: RefEqArc::new(ComputePipelineData {
                device,
                vk_pipeline,
                root_sig,
            }),
        }
    }

    pub fn vk_pipeline(&self) -> vk::Pipeline {
        self.data.vk_pipeline
    }

    pub(crate) fn root_sig(&self) -> &RootSig {
        &self.data.root_sig
    }
}

impl Drop for ComputePipelineData {
    fn drop(&mut self) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.destroy_pipeline(self.vk_pipeline, None);
        }
    }
}

/// Implementation of `RenderPipelineBuilder` for Vulkan.
#[derive(Debug)]
pub struct RenderPipelineBuilder {
    device: DeviceRef,
    vertex_shader: Option<(Library, String)>,
    fragment_shader: Option<(Library, String)>,
    root_sig: Option<RootSig>,
    render_pass: Option<(RenderPass, base::SubpassIndex)>,
    vertex_buffers: Vec<Option<VertexBufferBindingBuilder>>,
    vertex_attrs: Vec<Option<vk::VertexInputAttributeDescription>>,
    topology: vk::PrimitiveTopology,
    rasterizer: Option<RasterizerBuilder>,
}

zangfx_impl_object! { RenderPipelineBuilder: dyn base::RenderPipelineBuilder, dyn (crate::Debug) }

impl RenderPipelineBuilder {
    crate fn new(device: DeviceRef) -> Self {
        Self {
            device,
            vertex_shader: None,
            fragment_shader: None,
            root_sig: None,
            render_pass: None,
            vertex_buffers: Vec::new(),
            vertex_attrs: Vec::new(),
            // No default value is defined for `topology`
            topology: vk::PrimitiveTopology::PointList,
            rasterizer: None,
        }
    }
}

impl base::RenderPipelineBuilder for RenderPipelineBuilder {
    fn vertex_shader(
        &mut self,
        library: &base::LibraryRef,
        entry_point: &str,
    ) -> &mut dyn base::RenderPipelineBuilder {
        let my_library: &Library = library.downcast_ref().expect("bad library type");
        self.vertex_shader = Some((my_library.clone(), entry_point.to_owned()));
        self
    }

    fn fragment_shader(
        &mut self,
        library: &base::LibraryRef,
        entry_point: &str,
    ) -> &mut dyn base::RenderPipelineBuilder {
        let my_library: &Library = library.downcast_ref().expect("bad library type");
        self.fragment_shader = Some((my_library.clone(), entry_point.to_owned()));
        self
    }

    fn root_sig(&mut self, v: &base::RootSigRef) -> &mut dyn base::RenderPipelineBuilder {
        let my_root_sig: &RootSig = v.downcast_ref().expect("bad root signature type");
        self.root_sig = Some(my_root_sig.clone());
        self
    }

    fn render_pass(
        &mut self,
        v: &base::RenderPassRef,
        subpass: base::SubpassIndex,
    ) -> &mut dyn base::RenderPipelineBuilder {
        let render_pass: &RenderPass = v.downcast_ref().expect("bad render pass type");
        self.render_pass = Some((render_pass.clone(), subpass));
        self
    }

    fn vertex_buffer(
        &mut self,
        index: base::VertexBufferIndex,
        stride: base::DeviceSize,
    ) -> &mut dyn base::VertexBufferBinding {
        if self.vertex_buffers.len() <= index {
            self.vertex_buffers.resize(index + 1, None);
        }
        self.vertex_buffers[index] = Some(VertexBufferBindingBuilder::new(index as u32, stride));
        self.vertex_buffers[index].as_mut().unwrap()
    }

    fn vertex_attr(
        &mut self,
        index: base::VertexAttrIndex,
        buffer: base::VertexBufferIndex,
        offset: base::DeviceSize,
        format: base::VertexFormat,
    ) {
        if self.vertex_attrs.len() <= index {
            self.vertex_attrs.resize(index + 1, None);
        }
        self.vertex_attrs[index] = Some(vk::VertexInputAttributeDescription {
            location: index as u32,
            binding: buffer as u32,
            format: translate_vertex_format(format).expect("unsupported format"),
            offset: offset as u32,
        });
    }

    fn topology(&mut self, v: base::PrimitiveTopology) -> &mut dyn base::RenderPipelineBuilder {
        self.topology = match v {
            base::PrimitiveTopology::Points => vk::PrimitiveTopology::PointList,
            base::PrimitiveTopology::Lines => vk::PrimitiveTopology::LineList,
            base::PrimitiveTopology::LineStrip => vk::PrimitiveTopology::LineStrip,
            base::PrimitiveTopology::Triangles => vk::PrimitiveTopology::TriangleList,
            base::PrimitiveTopology::TriangleStrip => vk::PrimitiveTopology::TriangleStrip,
        };
        self
    }

    fn rasterize(&mut self) -> &mut dyn base::Rasterizer {
        if self.rasterizer.is_none() {
            self.rasterizer = Some(RasterizerBuilder::new());
        }
        self.rasterizer.as_mut().unwrap()
    }

    fn build(&mut self) -> Result<base::RenderPipelineRef> {
        let root_sig = self.root_sig.as_ref().expect("root_sig");

        let &(ref render_pass, subpass) = self.render_pass.as_ref().expect("render_pass");

        let mut dyn_states = Vec::new();

        let vertex_stage = self
            .vertex_shader
            .as_ref()
            .map(|s| new_shader_stage_description(base::ShaderStage::Vertex, &s.0, &s.1));

        let fragment_stage = self
            .fragment_shader
            .as_ref()
            .map(|s| new_shader_stage_description(base::ShaderStage::Fragment, &s.0, &s.1));

        let stages: Vec<vk::PipelineShaderStageCreateInfo> = [&vertex_stage, &fragment_stage]
            .iter()
            .filter_map(|s| s.as_ref().map(|s| s.0.clone()))
            .collect();

        let vertex_buffers: Vec<_> = self
            .vertex_buffers
            .iter()
            .filter_map(|vb| vb.as_ref().map(|vb| vb.vk_binding()))
            .collect();

        let vertex_attrs: Vec<_> = self
            .vertex_attrs
            .iter()
            .filter_map(|va| va.clone())
            .collect();

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo {
            s_type: vk::StructureType::PipelineVertexInputStateCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineVertexInputStateCreateFlags::empty(),
            vertex_binding_description_count: vertex_buffers.len() as u32,
            p_vertex_binding_descriptions: vertex_buffers.as_ptr(),
            vertex_attribute_description_count: vertex_attrs.len() as u32,
            p_vertex_attribute_descriptions: vertex_attrs.as_ptr(),
        };

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
            s_type: vk::StructureType::PipelineInputAssemblyStateCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            topology: self.topology,
            primitive_restart_enable: vk::VK_TRUE,
        };

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            s_type: vk::StructureType::PipelineViewportStateCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineViewportStateCreateFlags::empty(),
            viewport_count: 1,
            p_viewports: crate::null(),
            scissor_count: 1,
            p_scissors: crate::null(),
        };

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PipelineRasterizationStateCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: vk::VK_FALSE,
            rasterizer_discard_enable: vk::VK_TRUE,
            polygon_mode: vk::PolygonMode::Fill,
            cull_mode: vk::CULL_MODE_NONE,
            front_face: vk::FrontFace::CounterClockwise,
            depth_bias_enable: vk::VK_FALSE,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        };

        let mut vk_info = vk::GraphicsPipelineCreateInfo {
            s_type: vk::StructureType::GraphicsPipelineCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineCreateFlags::empty(),
            stage_count: stages.len() as u32,
            p_stages: stages.as_ptr(),
            p_vertex_input_state: &vertex_input_state,
            p_input_assembly_state: &input_assembly_state,
            p_tessellation_state: crate::null(),
            // May be overwritten by `LlRasterizer::populate` later
            p_viewport_state: &viewport_state,
            // May be overwritten by `LlRasterizer::populate` later
            p_rasterization_state: &rasterization_state,
            // Set by `LlRasterizer::populate` later
            p_multisample_state: crate::null(),
            // Set by `LlRasterizer::populate` later
            p_depth_stencil_state: crate::null(),
            // Set by `LlRasterizer::populate` later
            p_color_blend_state: crate::null(),
            // Set from `dyn_states` later
            p_dynamic_state: crate::null(),
            layout: root_sig.vk_pipeline_layout(),
            render_pass: render_pass.vk_render_pass(),
            subpass: subpass as u32,
            base_pipeline_handle: vk::Pipeline::null(),
            base_pipeline_index: 0,
        };

        // `ll_rasterizer` must live long enough or we will be passing
        // dangling pointers of various structs
        let ll_rasterizer;
        let partial_states;
        if let Some(ref r) = self.rasterizer {
            let num_color_attachments = render_pass.num_color_attachments(subpass);
            ll_rasterizer = LlRasterizer::new(r, num_color_attachments);
            ll_rasterizer.populate(&mut vk_info, &mut dyn_states);

            partial_states = ll_rasterizer.partial_states();
        } else {
            partial_states = RasterizerPartialStates::new();
        }

        let dynamic_state = vk::PipelineDynamicStateCreateInfo {
            s_type: vk::StructureType::PipelineDynamicStateCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineDynamicStateCreateFlags::empty(),
            dynamic_state_count: dyn_states.len() as u32,
            p_dynamic_states: dyn_states.as_ptr(),
        };
        vk_info.p_dynamic_state = &dynamic_state;

        let cache = vk::PipelineCache::null();

        let vk_device = self.device.vk_device();
        let vk_pipeline = unsafe { vk_device.create_graphics_pipelines(cache, &[vk_info], None) }
            .map_err(|e| translate_pipeline_creation_error_unwrap(&self.device, e))?[0];

        Ok(unsafe {
            RenderPipeline::from_raw(
                self.device.clone(),
                vk_pipeline,
                root_sig.clone(),
                partial_states,
            )
        }.into())
    }
}

/// Implementation of `VertexBufferBinding` for Vulkan.
#[derive(Debug, Clone)]
struct VertexBufferBindingBuilder {
    vk_binding: vk::VertexInputBindingDescription,
}

zangfx_impl_object! { VertexBufferBindingBuilder: dyn base::VertexBufferBinding, dyn (crate::Debug) }

impl VertexBufferBindingBuilder {
    fn new(binding: u32, stride: base::DeviceSize) -> Self {
        Self {
            vk_binding: vk::VertexInputBindingDescription {
                binding,
                stride: stride as u32,
                input_rate: vk::VertexInputRate::Vertex,
            },
        }
    }

    fn vk_binding(&self) -> vk::VertexInputBindingDescription {
        self.vk_binding.clone()
    }
}

impl base::VertexBufferBinding for VertexBufferBindingBuilder {
    fn set_rate(&mut self, rate: base::VertexInputRate) -> &mut dyn base::VertexBufferBinding {
        self.vk_binding.input_rate = match rate {
            base::VertexInputRate::Vertex => vk::VertexInputRate::Vertex,
            base::VertexInputRate::Instance => vk::VertexInputRate::Instance,
        };
        self
    }
}

#[derive(Debug)]
struct LlRasterizer<'a> {
    builder: &'a RasterizerBuilder,
    viewport_state: vk::PipelineViewportStateCreateInfo,
    static_scissors: Option<Vec<vk::Rect2D>>,
    rasterization_state: vk::PipelineRasterizationStateCreateInfo,
    multisample_state: vk::PipelineMultisampleStateCreateInfo,
    depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo,
    color_blend_state: vk::PipelineColorBlendStateCreateInfo,
    color_attachments: Vec<vk::PipelineColorBlendAttachmentState>,
}

impl<'a> LlRasterizer<'a> {
    fn new(builder: &'a RasterizerBuilder, num_color_attachments: usize) -> Self {
        use std::iter::repeat;

        let mut viewport_state = vk::PipelineViewportStateCreateInfo {
            s_type: vk::StructureType::PipelineViewportStateCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineViewportStateCreateFlags::empty(),
            viewport_count: builder.num_viewports as u32,
            p_viewports: crate::null(),
            scissor_count: builder.num_viewports as u32,
            p_scissors: crate::null(),
        };

        // Make entire the scissor rect array dynamic if at least one of them
        // is dynamic
        let static_scissors;
        let is_scissor_dynamic = builder
            .scissors
            .iter()
            .take(builder.num_viewports)
            .any(|s| s.is_dynamic());
        if is_scissor_dynamic {
            static_scissors = None;
        } else {
            static_scissors = Some({
                let default = Static(translate_rect2d_u32(&Rect2D::<u32>::all()));
                let scissors: Vec<_> = builder
                    .scissors
                    .iter()
                    .cloned()   // `Vec<StaticOrDynamic<vk::Rect2D>>`
                    .chain(repeat(default))
                    .take(builder.num_viewports)
                    .map(|s_or_d| s_or_d.static_value().unwrap())
                    .map(clip_rect2d_u31)
                    .collect();
                viewport_state.p_scissors = scissors.as_ptr();
                scissors
            });
        }

        let mut rasterization_state = vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PipelineRasterizationStateCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: translate_bool(builder.depth_clamp_enable),
            rasterizer_discard_enable: vk::VK_FALSE,
            polygon_mode: builder.polygon_mode,
            cull_mode: builder.cull_mode,
            front_face: builder.front_face,
            depth_bias_enable: translate_bool(builder.depth_bias.is_some()),
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        };

        if let Some(Static(ref bias)) = builder.depth_bias {
            rasterization_state.depth_bias_constant_factor = bias.constant_factor;
            rasterization_state.depth_bias_clamp = bias.clamp;
            rasterization_state.depth_bias_slope_factor = bias.slope_factor;
        }

        let multisample_state = vk::PipelineMultisampleStateCreateInfo {
            s_type: vk::StructureType::PipelineMultisampleStateCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineMultisampleStateCreateFlags::empty(),
            rasterization_samples: builder.rasterization_samples,
            sample_shading_enable: vk::VK_FALSE,
            min_sample_shading: 0.0,
            p_sample_mask: crate::null(),
            alpha_to_coverage_enable: translate_bool(builder.alpha_to_coverage_enable),
            alpha_to_one_enable: vk::VK_FALSE,
        };

        let stencil_test_enable = builder.stencil_ops.iter().any(|ops| {
            ops.fail_op != vk::StencilOp::Keep
                || ops.pass_op != vk::StencilOp::Keep
                || ops.depth_fail_op != vk::StencilOp::Keep
        });

        let mut depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
            s_type: vk::StructureType::PipelineDepthStencilStateCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineDepthStencilStateCreateFlags::empty(),
            depth_test_enable: translate_bool(builder.depth_test_enable),
            depth_write_enable: translate_bool(builder.depth_write_enable),
            depth_compare_op: builder.depth_compare_op,
            depth_bounds_test_enable: translate_bool(builder.depth_bounds.is_some()),
            stencil_test_enable: translate_bool(stencil_test_enable),
            front: builder.stencil_ops[0],
            back: builder.stencil_ops[1],
            min_depth_bounds: 0.0,
            max_depth_bounds: 0.0,
        };

        if let Some(Static(ref bounds)) = builder.depth_bounds {
            depth_stencil_state.min_depth_bounds = bounds.start;
            depth_stencil_state.max_depth_bounds = bounds.end;
        }

        let default = RasterizerColorTargetBuilder::new();
        let color_attachments: Vec<_> = builder
            .color_targets
            .iter()
            .chain(repeat(&default))
            .map(|target| target.vk_state.clone())
            .take(num_color_attachments)
            .collect();

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
            s_type: vk::StructureType::PipelineColorBlendStateCreateInfo,
            p_next: crate::null(),
            flags: vk::PipelineColorBlendStateCreateFlags::empty(),
            logic_op_enable: vk::VK_FALSE,
            logic_op: vk::LogicOp::Clear,
            attachment_count: num_color_attachments as u32,
            p_attachments: color_attachments.as_ptr(),
            blend_constants: [0.0; 4],
        };

        Self {
            builder,
            viewport_state,
            static_scissors,
            rasterization_state,
            multisample_state,
            depth_stencil_state,
            color_blend_state,
            color_attachments,
        }
    }

    fn populate(
        &self,
        vk_info: &mut vk::GraphicsPipelineCreateInfo,
        dyn_states: &mut Vec<vk::DynamicState>,
    ) {
        vk_info.p_viewport_state = &self.viewport_state;
        dyn_states.push(vk::DynamicState::Viewport);
        if self.static_scissors.is_none() {
            dyn_states.push(vk::DynamicState::Scissor);
        }
        // TODO: set partial scissor static state

        vk_info.p_rasterization_state = &self.rasterization_state;
        if let Some(Dynamic) = self.builder.depth_bias {
            dyn_states.push(vk::DynamicState::DepthBias);
        }

        vk_info.p_multisample_state = &self.multisample_state;

        vk_info.p_depth_stencil_state = &self.depth_stencil_state;
        dyn_states.push(vk::DynamicState::StencilReference);

        vk_info.p_color_blend_state = &self.color_blend_state;
        dyn_states.push(vk::DynamicState::BlendConstants);
    }

    fn partial_states(&self) -> RasterizerPartialStates {
        let scissors;
        if self.static_scissors.is_none() {
            scissors = self
                .builder
                .scissors
                .iter()
                .take(self.builder.num_viewports)
                .enumerate()
                .filter_map(|(i, scissor)| scissor.static_value().map(|rect| (i as u32, rect)))
                .collect();
        } else {
            scissors = Vec::new();
        }

        RasterizerPartialStates { scissors }
    }
}

/// A part of rasterizer states that must be set via render commands when the
/// pipeline is bound.
#[derive(Debug, Clone, Default)]
struct RasterizerPartialStates {
    scissors: Vec<(u32, vk::Rect2D)>,
}

impl RasterizerPartialStates {
    fn new() -> Self {
        Self::default()
    }
}

/// Implementation of `RasterizerBuilder` for Vulkan.
#[derive(Debug)]
struct RasterizerBuilder {
    num_viewports: usize,
    scissors: Vec<base::StaticOrDynamic<vk::Rect2D>>,
    cull_mode: vk::CullModeFlags,
    front_face: vk::FrontFace,
    depth_clamp_enable: bool,
    polygon_mode: vk::PolygonMode,
    alpha_to_coverage_enable: bool,
    rasterization_samples: vk::SampleCountFlags,
    depth_bias: Option<base::StaticOrDynamic<base::DepthBias>>,
    depth_write_enable: bool,
    depth_test_enable: bool,
    depth_compare_op: vk::CompareOp,
    depth_bounds: Option<base::StaticOrDynamic<Range<f32>>>,
    stencil_ops: [vk::StencilOpState; 2],
    color_targets: Vec<RasterizerColorTargetBuilder>,
}

zangfx_impl_object! { RasterizerBuilder: dyn base::Rasterizer, dyn (crate::Debug) }

impl RasterizerBuilder {
    fn new() -> Self {
        Self {
            num_viewports: 1,
            scissors: Vec::new(),
            cull_mode: vk::CULL_MODE_NONE,
            front_face: vk::FrontFace::CounterClockwise,
            depth_clamp_enable: false,
            polygon_mode: vk::PolygonMode::Fill,
            alpha_to_coverage_enable: false,
            rasterization_samples: vk::SAMPLE_COUNT_1_BIT,
            depth_bias: None,
            depth_write_enable: false,
            depth_test_enable: false,
            depth_compare_op: vk::CompareOp::Always,
            depth_bounds: None,
            stencil_ops: [vk::StencilOpState {
                fail_op: vk::StencilOp::Keep,
                pass_op: vk::StencilOp::Keep,
                depth_fail_op: vk::StencilOp::Keep,
                compare_op: vk::CompareOp::Always,
                compare_mask: 0u32,
                write_mask: 0u32,
                reference: 0u32,
            }; 2],
            color_targets: Vec::new(),
        }
    }
}

impl base::Rasterizer for RasterizerBuilder {
    fn set_num_viewports(&mut self, v: usize) -> &mut dyn base::Rasterizer {
        self.num_viewports = v;
        self
    }

    fn set_scissors(
        &mut self,
        start_viewport: base::ViewportIndex,
        v: &[base::StaticOrDynamic<Rect2D<u32>>],
    ) -> &mut dyn base::Rasterizer {
        if v.len() == 0 {
            return self;
        }
        if start_viewport + v.len() > self.scissors.len() {
            let default = Static(translate_rect2d_u32(&Rect2D::<u32>::all()));
            self.scissors.resize(start_viewport + v.len(), default);
        }
        for (i, v) in v.iter().enumerate() {
            self.scissors[i + start_viewport] = v.as_ref().map(translate_rect2d_u32);
        }
        self
    }

    fn set_cull_mode(&mut self, v: base::CullMode) -> &mut dyn base::Rasterizer {
        self.cull_mode = match v {
            base::CullMode::None => vk::CULL_MODE_NONE,
            base::CullMode::Front => vk::CULL_MODE_FRONT_BIT,
            base::CullMode::Back => vk::CULL_MODE_BACK_BIT,
        };
        self
    }

    fn set_front_face(&mut self, v: base::Winding) -> &mut dyn base::Rasterizer {
        self.front_face = match v {
            base::Winding::Clockwise => vk::FrontFace::Clockwise,
            base::Winding::CounterClockwise => vk::FrontFace::CounterClockwise,
        };
        self
    }

    fn set_depth_clip_mode(&mut self, v: base::DepthClipMode) -> &mut dyn base::Rasterizer {
        self.depth_clamp_enable = v == base::DepthClipMode::Clamp;
        self
    }

    fn set_triangle_fill_mode(&mut self, v: base::TriangleFillMode) -> &mut dyn base::Rasterizer {
        self.polygon_mode = match v {
            base::TriangleFillMode::Fill => vk::PolygonMode::Fill,
            base::TriangleFillMode::Line => vk::PolygonMode::Line,
        };
        self
    }

    fn set_depth_bias(
        &mut self,
        v: Option<base::StaticOrDynamic<base::DepthBias>>,
    ) -> &mut dyn base::Rasterizer {
        self.depth_bias = v;
        self
    }

    fn set_alpha_to_coverage(&mut self, v: bool) -> &mut dyn base::Rasterizer {
        self.alpha_to_coverage_enable = v;
        self
    }

    fn set_sample_count(&mut self, v: u32) -> &mut dyn base::Rasterizer {
        self.rasterization_samples = translate_sample_count(v);
        self
    }

    fn set_depth_write(&mut self, v: bool) -> &mut dyn base::Rasterizer {
        self.depth_write_enable = v;
        self
    }

    fn set_depth_test(&mut self, v: base::CmpFn) -> &mut dyn base::Rasterizer {
        self.depth_test_enable = v != base::CmpFn::Always;
        self.depth_compare_op = translate_compare_op(v);
        self
    }

    fn set_stencil_ops(&mut self, front_back: [base::StencilOps; 2]) -> &mut dyn base::Rasterizer {
        for i in 0..2 {
            self.stencil_ops[i].fail_op = translate_stencil_op(front_back[i].stencil_fail);
            self.stencil_ops[i].pass_op = translate_stencil_op(front_back[i].pass);
            self.stencil_ops[i].depth_fail_op = translate_stencil_op(front_back[i].depth_fail);
            self.stencil_ops[i].compare_op = translate_compare_op(front_back[i].stencil_test);
        }
        self
    }

    fn set_stencil_masks(
        &mut self,
        front_back: [base::StencilMasks; 2],
    ) -> &mut dyn base::Rasterizer {
        for i in 0..2 {
            self.stencil_ops[i].compare_mask = front_back[i].read;
            self.stencil_ops[i].write_mask = front_back[i].write;
        }
        self
    }

    fn set_depth_bounds(
        &mut self,
        v: Option<base::StaticOrDynamic<Range<f32>>>,
    ) -> &mut dyn base::Rasterizer {
        self.depth_bounds = v;
        self
    }

    fn color_target(
        &mut self,
        index: base::RenderSubpassColorTargetIndex,
    ) -> &mut dyn base::RasterizerColorTarget {
        if self.color_targets.len() <= index {
            self.color_targets
                .resize(index + 1, RasterizerColorTargetBuilder::new());
        }
        &mut self.color_targets[index]
    }
}

/// Implementation of `RasterizerColorTargetBuilder` for Vulkan.
#[derive(Debug, Clone)]
struct RasterizerColorTargetBuilder {
    vk_state: vk::PipelineColorBlendAttachmentState,
}

zangfx_impl_object! { RasterizerColorTargetBuilder: dyn base::RasterizerColorTarget, dyn (crate::Debug) }

impl RasterizerColorTargetBuilder {
    fn new() -> Self {
        Self {
            vk_state: vk::PipelineColorBlendAttachmentState {
                blend_enable: vk::VK_FALSE,
                src_color_blend_factor: vk::BlendFactor::One,
                dst_color_blend_factor: vk::BlendFactor::Zero,
                color_blend_op: vk::BlendOp::Add,
                src_alpha_blend_factor: vk::BlendFactor::One,
                dst_alpha_blend_factor: vk::BlendFactor::Zero,
                alpha_blend_op: vk::BlendOp::Add,
                color_write_mask: vk::COLOR_COMPONENT_R_BIT
                    | vk::COLOR_COMPONENT_G_BIT
                    | vk::COLOR_COMPONENT_B_BIT
                    | vk::COLOR_COMPONENT_A_BIT,
            },
        }
    }
}

impl base::RasterizerColorTarget for RasterizerColorTargetBuilder {
    fn set_write_mask(
        &mut self,
        v: base::ColorChannelFlags,
    ) -> &mut dyn base::RasterizerColorTarget {
        self.vk_state.color_write_mask = translate_color_channel_flags(v);
        self
    }

    fn set_blending(&mut self, v: bool) -> &mut dyn base::RasterizerColorTarget {
        self.vk_state.blend_enable = translate_bool(v);
        self
    }

    fn set_src_alpha_factor(
        &mut self,
        v: base::BlendFactor,
    ) -> &mut dyn base::RasterizerColorTarget {
        self.vk_state.src_alpha_blend_factor = translate_blend_factor(v);
        self
    }

    fn set_src_rgb_factor(&mut self, v: base::BlendFactor) -> &mut dyn base::RasterizerColorTarget {
        self.vk_state.src_color_blend_factor = translate_blend_factor(v);
        self
    }

    fn set_dst_alpha_factor(
        &mut self,
        v: base::BlendFactor,
    ) -> &mut dyn base::RasterizerColorTarget {
        self.vk_state.dst_alpha_blend_factor = translate_blend_factor(v);
        self
    }

    fn set_dst_rgb_factor(&mut self, v: base::BlendFactor) -> &mut dyn base::RasterizerColorTarget {
        self.vk_state.dst_color_blend_factor = translate_blend_factor(v);
        self
    }

    fn set_alpha_op(&mut self, v: base::BlendOp) -> &mut dyn base::RasterizerColorTarget {
        self.vk_state.alpha_blend_op = translate_blend_op(v);
        self
    }

    fn set_rgb_op(&mut self, v: base::BlendOp) -> &mut dyn base::RasterizerColorTarget {
        self.vk_state.color_blend_op = translate_blend_op(v);
        self
    }
}

fn translate_stencil_op(value: base::StencilOp) -> vk::StencilOp {
    match value {
        base::StencilOp::Keep => vk::StencilOp::Keep,
        base::StencilOp::Zero => vk::StencilOp::Zero,
        base::StencilOp::Replace => vk::StencilOp::Replace,
        base::StencilOp::IncrementAndClamp => vk::StencilOp::IncrementAndClamp,
        base::StencilOp::DecrementAndClamp => vk::StencilOp::DecrementAndClamp,
        base::StencilOp::Invert => vk::StencilOp::Invert,
        base::StencilOp::IncrementAndWrap => vk::StencilOp::IncrementAndWrap,
        base::StencilOp::DecrementAndWrap => vk::StencilOp::DecrementAndWrap,
    }
}

fn translate_blend_factor(value: base::BlendFactor) -> vk::BlendFactor {
    match value {
        base::BlendFactor::Zero => vk::BlendFactor::Zero,
        base::BlendFactor::One => vk::BlendFactor::One,
        base::BlendFactor::SrcColor => vk::BlendFactor::SrcColor,
        base::BlendFactor::OneMinusSrcColor => vk::BlendFactor::OneMinusSrcColor,
        base::BlendFactor::SrcAlpha => vk::BlendFactor::SrcAlpha,
        base::BlendFactor::OneMinusSrcAlpha => vk::BlendFactor::OneMinusSrcAlpha,
        base::BlendFactor::DstColor => vk::BlendFactor::DstColor,
        base::BlendFactor::OneMinusDstColor => vk::BlendFactor::OneMinusDstColor,
        base::BlendFactor::DstAlpha => vk::BlendFactor::DstAlpha,
        base::BlendFactor::OneMinusDstAlpha => vk::BlendFactor::OneMinusDstAlpha,
        base::BlendFactor::ConstantColor => vk::BlendFactor::ConstantColor,
        base::BlendFactor::OneMinusConstantColor => vk::BlendFactor::OneMinusConstantColor,
        base::BlendFactor::ConstantAlpha => vk::BlendFactor::ConstantAlpha,
        base::BlendFactor::OneMinusConstantAlpha => vk::BlendFactor::OneMinusConstantAlpha,
        base::BlendFactor::SrcAlphaSaturated => vk::BlendFactor::SrcAlphaSaturate,
        base::BlendFactor::Src1Color => vk::BlendFactor::Src1Color,
        base::BlendFactor::OneMinusSrc1Color => vk::BlendFactor::OneMinusSrc1Color,
        base::BlendFactor::Src1Alpha => vk::BlendFactor::Src1Alpha,
        base::BlendFactor::OneMinusSrc1Alpha => vk::BlendFactor::OneMinusSrc1Alpha,
    }
}

fn translate_blend_op(value: base::BlendOp) -> vk::BlendOp {
    match value {
        base::BlendOp::Add => vk::BlendOp::Add,
        base::BlendOp::Subtract => vk::BlendOp::Subtract,
        base::BlendOp::ReverseSubtract => vk::BlendOp::ReverseSubtract,
        base::BlendOp::Min => vk::BlendOp::Min,
        base::BlendOp::Max => vk::BlendOp::Max,
    }
}

/// Implementation of `RenderPipeline` for Vulkan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderPipeline {
    data: RefEqArc<RenderPipelineData>,
}

zangfx_impl_handle! { RenderPipeline, base::RenderPipelineRef }

#[derive(Debug)]
struct RenderPipelineData {
    device: DeviceRef,
    vk_pipeline: vk::Pipeline,
    root_sig: RootSig,
    partial_states: RasterizerPartialStates,
}

impl RenderPipeline {
    unsafe fn from_raw(
        device: DeviceRef,
        vk_pipeline: vk::Pipeline,
        root_sig: RootSig,
        partial_states: RasterizerPartialStates,
    ) -> Self {
        Self {
            data: RefEqArc::new(RenderPipelineData {
                device,
                vk_pipeline,
                root_sig,
                partial_states,
            }),
        }
    }

    pub fn vk_pipeline(&self) -> vk::Pipeline {
        self.data.vk_pipeline
    }

    pub(crate) fn root_sig(&self) -> &RootSig {
        &self.data.root_sig
    }

    pub(crate) unsafe fn encode_partial_states(&self, vk_cmd_buffer: vk::CommandBuffer) {
        let vk_device: &crate::AshDevice = self.data.device.vk_device();

        for &(i, ref rect) in self.data.partial_states.scissors.iter() {
            vk_device
                .fp_v1_0()
                .cmd_set_scissor(vk_cmd_buffer, i, 1, rect);
        }
    }
}

impl Drop for RenderPipelineData {
    fn drop(&mut self) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.destroy_pipeline(self.vk_pipeline, None);
        }
    }
}
