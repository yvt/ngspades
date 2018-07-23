//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;
use std::ops::Range;
use zangfx_metal_rs as metal;

use zangfx_base as base;
use zangfx_base::StaticOrDynamic::*;
use zangfx_base::{Error, ErrorKind, Result};
use zangfx_base::{zangfx_impl_object, interfaces, vtable_for, zangfx_impl_handle};
use zangfx_common::BinaryInteger;
use crate::arg::table::ArgTable;
use crate::arg::rootsig::RootSig;
use crate::buffer::Buffer;
use crate::shader::{Library, ShaderVertexAttrInfo};
use crate::renderpass::RenderPass;
use crate::formats::translate_vertex_format;

use crate::utils::{clip_scissor_rect, nil_error, translate_cmp_fn, translate_scissor_rect,
            translate_viewport, OCPtr};

/// Implementation of `RenderPipelineBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct RenderPipelineBuilder {
    /// A reference to a `MTLDevice`. We are not required to maintain a strong
    /// reference. (See the base interface's documentation)
    metal_device: metal::MTLDevice,

    vertex_shader: Option<(Library, String)>,
    fragment_shader: Option<(Library, String)>,
    root_sig: Option<RootSig>,
    render_pass: Option<(RenderPass, base::SubpassIndex)>,
    topology: Option<base::PrimitiveTopology>,
    vertex_buffers: Vec<Option<VertexBufferBinding>>,
    vertex_attrs: Vec<Option<VertexAttrBinding>>,
    rasterizer: Option<Rasterizer>,

    label: Option<String>,
}

zangfx_impl_object! { RenderPipelineBuilder: dyn base::RenderPipelineBuilder, dyn crate::Debug, dyn base::SetLabel }

unsafe impl Send for RenderPipelineBuilder {}
unsafe impl Sync for RenderPipelineBuilder {}

impl RenderPipelineBuilder {
    /// Construct a `RenderPipelineBuilder`.
    ///
    /// Ir's up to the caller to maintain the lifetime of `metal_device`.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device,
            vertex_shader: None,
            fragment_shader: None,
            root_sig: None,
            render_pass: None,
            topology: None,
            vertex_buffers: Vec::new(),
            vertex_attrs: Vec::new(),
            rasterizer: None,
            label: None,
        }
    }
}

impl base::SetLabel for RenderPipelineBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
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
        let my_render_pass: &RenderPass = v.downcast_ref().expect("bad render pass type");
        self.render_pass = Some((my_render_pass.clone(), subpass));
        self
    }

    fn vertex_buffer(
        &mut self,
        index: base::VertexBufferIndex,
        stride: base::DeviceSize,
    ) -> &mut dyn base::VertexBufferBinding {
        assert!(
            index < crate::MAX_NUM_VERTEX_BUFFERS,
            "index exceeds implementation limit"
        );
        if self.vertex_buffers.len() <= index {
            self.vertex_buffers.resize(index + 1, None);
        }
        self.vertex_buffers[index] = Some(VertexBufferBinding::new(stride));
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
        self.vertex_attrs[index] = Some(VertexAttrBinding::new(buffer, offset, format));
    }

    fn topology(&mut self, v: base::PrimitiveTopology) -> &mut dyn base::RenderPipelineBuilder {
        self.topology = Some(v);
        self
    }

    fn rasterize(&mut self) -> &mut dyn base::Rasterizer {
        if self.rasterizer.is_none() {
            self.rasterizer = Some(Rasterizer::new());
        }
        self.rasterizer.as_mut().unwrap()
    }

    fn build(&mut self) -> Result<base::RenderPipelineRef> {
        let root_sig = self.root_sig
            .as_ref()
            .expect("root_sig");

        let vertex_shader = self.vertex_shader
            .as_ref()
            .expect("vertex_shader");

        let &(ref render_pass, subpass_index) = self.render_pass
            .as_ref()
            .expect("render_pass");

        let metal_desc = unsafe {
            OCPtr::from_raw(metal::MTLRenderPipelineDescriptor::alloc().init())
                .ok_or_else(|| nil_error("MTLRenderPipelineDescriptor alloc"))?
        };

        // Setup MTLVertexDescriptor
        let metal_vertex_desc = metal_desc.vertex_descriptor();
        assert!(!metal_vertex_desc.is_null());

        // Vertex buffer indices are straightly mapped to Metal vertex buffer
        // argument table indices.
        // (In NgsGFX we used a minimal number of argument table entries. In
        // contrast with NgsGFX, arguments in ZanGFX are entirely passed via
        // indirect arguments. This leaves a plenty of free argument table
        // entries available for vertex buffers.)
        let vb_start_index = root_sig.gfx_vertex_buffer_index();

        let metal_va_array = metal_vertex_desc.attributes();
        for (i, vertex_attr) in self.vertex_attrs.iter().enumerate() {
            if let &Some(ref vertex_attr) = vertex_attr {
                let metal_va_desc = metal_va_array.object_at(i as _);
                assert!(!metal_va_desc.is_null());
                vertex_attr.populate(metal_va_desc, vb_start_index as usize);
            }
        }

        let metal_vbl_array = metal_vertex_desc.layouts();
        let mut vb_used = 0u32;
        for (i, vertex_buffer) in self.vertex_buffers.iter().enumerate() {
            if let &Some(ref vertex_buffer) = vertex_buffer {
                let metal_vbl_desc: metal::MTLVertexBufferLayoutDescriptor =
                    metal_vbl_array.object_at(i + vb_start_index as usize);
                assert!(!metal_vbl_desc.is_null());
                vertex_buffer.populate(metal_vbl_desc);
                vb_used.set_bit(i as u32);
            }
        }

        let shader_va_infos = self.vertex_attrs.iter().enumerate().filter_map(
            |(i, vertex_attr)| {
                vertex_attr.as_ref().map(|vertex_attr| {
                    let vertex_buffer = self.vertex_buffers
                        .get(vertex_attr.buffer)
                        .unwrap_or(&None)
                        .as_ref()
                        .expect("vertex buffer binding is missing");
                    ShaderVertexAttrInfo {
                        binding: i,
                        msl_buffer_index: vertex_attr.buffer + vb_start_index as usize,
                        offset: vertex_attr.offset as u32,
                        stride: vertex_buffer.stride as u32,
                        input_rate: vertex_buffer.step_fn,
                    }
                })
            },
        );

        let vertex_fn = vertex_shader.0.new_metal_function(
            &vertex_shader.1,
            base::ShaderStage::Vertex,
            root_sig,
            shader_va_infos,
            self.metal_device,
            &self.label,
        )?;
        metal_desc.set_vertex_function(*vertex_fn);

        // Populate rasterizer states
        let rast_partial_states;
        if let Some(ref rasterizer) = self.rasterizer {
            // Fragment shader is mandatory only if rasterization is enabled
            let fragment_shader = self.fragment_shader
                .as_ref()
                .expect("fragment_shader");

            let fragment_fn = fragment_shader.0.new_metal_function(
                &fragment_shader.1,
                base::ShaderStage::Fragment,
                root_sig,
                ::std::iter::empty(),
                self.metal_device,
                &self.label,
            )?;
            metal_desc.set_fragment_function(*fragment_fn);

            rast_partial_states = Some(rasterizer.populate(
                *metal_desc,
                render_pass,
                subpass_index,
                self.metal_device,
            )?);
        } else {
            rast_partial_states = None;
            metal_desc.set_rasterization_enabled(false);
        }

        let topology = self.topology
            .expect("topology");

        let prim_type = match topology {
            base::PrimitiveTopology::Points => metal::MTLPrimitiveType::Point,
            base::PrimitiveTopology::Lines => metal::MTLPrimitiveType::Line,
            base::PrimitiveTopology::LineStrip => metal::MTLPrimitiveType::LineStrip,
            base::PrimitiveTopology::Triangles => metal::MTLPrimitiveType::Triangle,
            base::PrimitiveTopology::TriangleStrip => metal::MTLPrimitiveType::TriangleStrip,
        };

        let topo_class = match topology {
            base::PrimitiveTopology::Points => metal::MTLPrimitiveTopologyClass::Point,
            base::PrimitiveTopology::Lines | base::PrimitiveTopology::LineStrip => {
                metal::MTLPrimitiveTopologyClass::Line
            }
            base::PrimitiveTopology::Triangles | base::PrimitiveTopology::TriangleStrip => {
                metal::MTLPrimitiveTopologyClass::Triangle
            }
        };
        metal_desc.set_input_primitive_topology(topo_class);

        if let Some(ref label) = self.label {
            metal_desc.set_label(label);
        }

        let metal_pipeline = self.metal_device
            .new_render_pipeline_state(*metal_desc)
            .map_err(|e| Error::with_detail(ErrorKind::Other, e))
            .and_then(|p| {
                OCPtr::new(p).ok_or_else(|| {
                    nil_error(
                        "MTLDevice newRenderPipelineStateWithDescriptor:options:reflection:error:",
                    )
                })
            })?;

        let data = RenderPipelineData {
            metal_pipeline,
            rast_partial_states,
            prim_type,
            vb_start_index,
            vb_used,
        };

        Ok(RenderPipeline {
            data: Arc::new(data),
        }.into())
    }
}

/// Implementation of `VertexBufferBinding` for Metal.
#[derive(Debug, Clone)]
struct VertexBufferBinding {
    step_fn: metal::MTLVertexStepFunction,
    stride: base::DeviceSize,
}

zangfx_impl_object! { VertexBufferBinding: dyn base::VertexBufferBinding, dyn crate::Debug }

impl VertexBufferBinding {
    fn new(stride: base::DeviceSize) -> Self {
        Self {
            step_fn: metal::MTLVertexStepFunction::PerVertex,
            stride,
        }
    }

    fn populate(&self, metal_desc: metal::MTLVertexBufferLayoutDescriptor) {
        metal_desc.set_step_function(self.step_fn);
        metal_desc.set_stride(self.stride);
    }
}

impl base::VertexBufferBinding for VertexBufferBinding {
    fn set_rate(&mut self, rate: base::VertexInputRate) -> &mut dyn base::VertexBufferBinding {
        self.step_fn = match rate {
            base::VertexInputRate::Vertex => metal::MTLVertexStepFunction::PerVertex,
            base::VertexInputRate::Instance => metal::MTLVertexStepFunction::PerInstance,
        };
        self
    }
}

/// Describes a vertex attribute.
#[derive(Debug, Clone)]
struct VertexAttrBinding {
    buffer: base::VertexBufferIndex,
    offset: base::DeviceSize,
    format: metal::MTLVertexFormat,
}

impl VertexAttrBinding {
    fn new(
        buffer: base::VertexBufferIndex,
        offset: base::DeviceSize,
        format: base::VertexFormat,
    ) -> Self {
        Self {
            buffer,
            offset,
            format: translate_vertex_format(format).expect("unsupported vertex format"),
        }
    }

    fn populate(&self, metal_desc: metal::MTLVertexAttributeDescriptor, vb_start_index: usize) {
        metal_desc.set_buffer_index((self.buffer + vb_start_index) as u64);
        metal_desc.set_offset(self.offset as u64);
        metal_desc.set_format(self.format);
    }
}

/// Implementation of `Rasterizer` for Metal.
#[derive(Debug, Clone)]
struct Rasterizer {
    scissor: base::StaticOrDynamic<base::Rect2D<u32>>,
    cull_mode: metal::MTLCullMode,
    front_face: metal::MTLWinding,
    depth_clip_mode: metal::MTLDepthClipMode,
    triangle_fill_mode: metal::MTLTriangleFillMode,
    depth_bias: Option<base::DepthBias>,
    alpha_to_coverage: bool,
    sample_count: u32,
    depth_write: bool,
    depth_test: metal::MTLCompareFunction,
    stencil_ops: [MetalStencilOps; 2],
    stencil_masks: [base::StencilMasks; 2],
    color_targets: Vec<RasterizerColorTarget>,
}

zangfx_impl_object! { Rasterizer: dyn base::Rasterizer, dyn crate::Debug }

/// A part of rasterizer states that must be set via render commands when the
/// pipeline is bound.
#[derive(Debug, Clone)]
struct RasterizerPartialStates {
    scissor: Option<base::Rect2D<u32>>,
    cull_mode: metal::MTLCullMode,
    front_face: metal::MTLWinding,
    depth_clip_mode: metal::MTLDepthClipMode,
    triangle_fill_mode: metal::MTLTriangleFillMode,
    depth_bias: Option<base::DepthBias>,
    depth_stencil: OCPtr<metal::MTLDepthStencilState>,
    compact_depth_stencil: CompactDsState,
}

impl Rasterizer {
    fn new() -> Self {
        Self {
            scissor: Static(base::Rect2D::all()),
            cull_mode: metal::MTLCullMode::None,
            front_face: metal::MTLWinding::CounterClockwise,
            depth_clip_mode: metal::MTLDepthClipMode::Clip,
            triangle_fill_mode: metal::MTLTriangleFillMode::Fill,
            depth_bias: None,
            alpha_to_coverage: false,
            sample_count: 1,
            depth_write: false,
            depth_test: metal::MTLCompareFunction::Always,
            stencil_ops: Default::default(),
            stencil_masks: Default::default(),
            color_targets: Vec::new(),
        }
    }

    fn populate(
        &self,
        metal_desc: metal::MTLRenderPipelineDescriptor,
        render_pass: &RenderPass,
        subpass_index: base::SubpassIndex,
        metal_device: metal::MTLDevice,
    ) -> Result<RasterizerPartialStates> {
        metal_desc.set_alpha_to_coverage_enabled(self.alpha_to_coverage);
        metal_desc.set_sample_count(self.sample_count as u64);

        // Construct a `MTLDepthStencilState`
        let metal_ds = unsafe { OCPtr::from_raw(metal::MTLDepthStencilDescriptor::alloc().init()) }
            .ok_or_else(|| nil_error("MTLDepthStencilDescriptor alloc"))?;

        metal_ds.set_depth_write_enabled(self.depth_write);
        metal_ds.set_depth_compare_function(self.depth_test);

        fn populate_stencil(
            ops: &MetalStencilOps,
            masks: &base::StencilMasks,
        ) -> Result<OCPtr<metal::MTLStencilDescriptor>> {
            let metal_desc = unsafe {
                OCPtr::from_raw(metal::MTLStencilDescriptor::alloc().init())
            }.ok_or_else(|| nil_error("MTLStencilDescriptor alloc"))?;

            metal_desc.set_stencil_compare_function(ops.compare);
            metal_desc.set_stencil_failure_operation(ops.stencil_failure);
            metal_desc.set_depth_failure_operation(ops.depth_failure);
            metal_desc.set_depth_stencil_pass_operation(ops.pass);
            metal_desc.set_read_mask(masks.read);
            metal_desc.set_write_mask(masks.write);

            Ok(metal_desc)
        }

        if self.stencil_ops[0] != MetalStencilOps::default() {
            metal_ds.set_front_face_stencil(*populate_stencil(
                &self.stencil_ops[0],
                &self.stencil_masks[0],
            )?);
        }
        if self.stencil_ops[1] != MetalStencilOps::default() {
            metal_ds.set_back_face_stencil(*populate_stencil(
                &self.stencil_ops[1],
                &self.stencil_masks[1],
            )?);
        }

        let depth_stencil = OCPtr::new(metal_device.new_depth_stencil_state(*metal_ds))
            .ok_or_else(|| nil_error("MTLDevice newDepthStencilStateWithDescriptor:"))?;

        let compact_depth_stencil = CompactDsState::from_rasterizer(self);

        // Setup attachments
        metal_desc.set_depth_attachment_pixel_format(render_pass.depth_format(subpass_index));
        metal_desc.set_stencil_attachment_pixel_format(render_pass.stencil_format(subpass_index));

        let metal_color_array = metal_desc.color_attachments();
        let color_target_default = RasterizerColorTarget::new();

        for i in 0..render_pass.num_color_attachments() {
            let metal_color = metal_color_array.object_at(i);
            let format = render_pass.color_format(subpass_index, i);
            metal_color.set_pixel_format(format);

            if format != metal::MTLPixelFormat::Invalid {
                self.color_targets
                    .get(i)
                    .unwrap_or(&color_target_default)
                    .populate(metal_color);
            }
        }

        Ok(RasterizerPartialStates {
            scissor: self.scissor.static_value(),
            cull_mode: self.cull_mode,
            front_face: self.front_face,
            depth_clip_mode: self.depth_clip_mode,
            triangle_fill_mode: self.triangle_fill_mode,
            depth_bias: self.depth_bias,
            depth_stencil,
            compact_depth_stencil,
        })
    }
}

impl base::Rasterizer for Rasterizer {
    fn set_num_viewports(&mut self, v: usize) -> &mut dyn base::Rasterizer {
        // Multiple viewport are not supported
        assert_eq!(v, 1);
        self
    }

    fn set_scissors(
        &mut self,
        start_viewport: base::ViewportIndex,
        v: &[base::StaticOrDynamic<base::Rect2D<u32>>],
    ) -> &mut dyn base::Rasterizer {
        // Multiple viewport are not supported
        if v.len() > 0 {
            assert_eq!(start_viewport, 0);
            assert_eq!(v.len(), 1);
            self.scissor = v[0];
        }
        self
    }

    fn set_cull_mode(&mut self, v: base::CullMode) -> &mut dyn base::Rasterizer {
        self.cull_mode = match v {
            base::CullMode::None => metal::MTLCullMode::None,
            base::CullMode::Back => metal::MTLCullMode::Back,
            base::CullMode::Front => metal::MTLCullMode::Front,
        };
        self
    }

    fn set_front_face(&mut self, v: base::Winding) -> &mut dyn base::Rasterizer {
        self.front_face = match v {
            base::Winding::Clockwise => metal::MTLWinding::Clockwise,
            base::Winding::CounterClockwise => metal::MTLWinding::CounterClockwise,
        };
        self
    }

    fn set_depth_clip_mode(&mut self, v: base::DepthClipMode) -> &mut dyn base::Rasterizer {
        self.depth_clip_mode = match v {
            base::DepthClipMode::Clip => metal::MTLDepthClipMode::Clip,
            base::DepthClipMode::Clamp => metal::MTLDepthClipMode::Clamp,
        };
        self
    }

    fn set_triangle_fill_mode(&mut self, v: base::TriangleFillMode) -> &mut dyn base::Rasterizer {
        self.triangle_fill_mode = match v {
            base::TriangleFillMode::Line => metal::MTLTriangleFillMode::Lines,
            base::TriangleFillMode::Fill => metal::MTLTriangleFillMode::Fill,
        };
        self
    }

    fn set_depth_bias(
        &mut self,
        v: Option<base::StaticOrDynamic<base::DepthBias>>,
    ) -> &mut dyn base::Rasterizer {
        self.depth_bias = v.unwrap_or(Static(Default::default())).static_value();
        self
    }

    fn set_alpha_to_coverage(&mut self, v: bool) -> &mut dyn base::Rasterizer {
        self.alpha_to_coverage = v;
        self
    }

    fn set_sample_count(&mut self, v: u32) -> &mut dyn base::Rasterizer {
        self.sample_count = v;
        self
    }

    fn set_depth_write(&mut self, v: bool) -> &mut dyn base::Rasterizer {
        self.depth_write = v;
        self
    }

    fn set_depth_test(&mut self, v: base::CmpFn) -> &mut dyn base::Rasterizer {
        self.depth_test = translate_cmp_fn(v);
        self
    }

    fn set_stencil_ops(&mut self, front_back: [base::StencilOps; 2]) -> &mut dyn base::Rasterizer {
        self.stencil_ops = [front_back[0].into(), front_back[1].into()];
        self
    }

    fn set_stencil_masks(&mut self, front_back: [base::StencilMasks; 2]) -> &mut dyn base::Rasterizer {
        self.stencil_masks = front_back;
        self
    }

    fn set_depth_bounds(
        &mut self,
        v: Option<base::StaticOrDynamic<Range<f32>>>,
    ) -> &mut dyn base::Rasterizer {
        assert_eq!(v, None);
        self
    }

    fn color_target(
        &mut self,
        index: base::RenderSubpassColorTargetIndex,
    ) -> &mut dyn base::RasterizerColorTarget {
        if self.color_targets.len() <= index {
            self.color_targets
                .resize(index + 1, RasterizerColorTarget::new());
        }
        &mut self.color_targets[index]
    }
}

/// Implementation of `RasterizerColorTarget` for Metal.
#[derive(Debug, Clone)]
struct RasterizerColorTarget {
    write_mask: metal::MTLColorWriteMask,
    blending: bool,
    src_alpha_factor: metal::MTLBlendFactor,
    src_rgb_factor: metal::MTLBlendFactor,
    dst_alpha_factor: metal::MTLBlendFactor,
    dst_rgb_factor: metal::MTLBlendFactor,
    alpha_op: metal::MTLBlendOperation,
    rgb_op: metal::MTLBlendOperation,
}

zangfx_impl_object! { RasterizerColorTarget: dyn base::RasterizerColorTarget, dyn crate::Debug }

impl RasterizerColorTarget {
    fn new() -> Self {
        Self {
            write_mask: metal::MTLColorWriteMaskAll,
            blending: false,
            src_alpha_factor: metal::MTLBlendFactor::One,
            src_rgb_factor: metal::MTLBlendFactor::One,
            dst_alpha_factor: metal::MTLBlendFactor::Zero,
            dst_rgb_factor: metal::MTLBlendFactor::Zero,
            alpha_op: metal::MTLBlendOperation::Add,
            rgb_op: metal::MTLBlendOperation::Add,
        }
    }

    fn populate(&self, metal_desc: metal::MTLRenderPipelineColorAttachmentDescriptor) {
        if self.blending {
            metal_desc.set_blending_enabled(true);
            metal_desc.set_source_rgb_blend_factor(self.src_rgb_factor);
            metal_desc.set_source_alpha_blend_factor(self.src_alpha_factor);
            metal_desc.set_destination_rgb_blend_factor(self.dst_rgb_factor);
            metal_desc.set_destination_alpha_blend_factor(self.dst_alpha_factor);
            metal_desc.set_rgb_blend_operation(self.rgb_op);
            metal_desc.set_alpha_blend_operation(self.alpha_op);
        }
        metal_desc.set_write_mask(self.write_mask);
    }
}

impl base::RasterizerColorTarget for RasterizerColorTarget {
    fn set_write_mask(&mut self, v: base::ColorChannelFlags) -> &mut dyn base::RasterizerColorTarget {
        let mut mask = metal::MTLColorWriteMaskNone;
        if v.intersects(base::ColorChannel::Red) {
            mask |= metal::MTLColorWriteMaskRed;
        }
        if v.intersects(base::ColorChannel::Green) {
            mask |= metal::MTLColorWriteMaskGreen;
        }
        if v.intersects(base::ColorChannel::Blue) {
            mask |= metal::MTLColorWriteMaskBlue;
        }
        if v.intersects(base::ColorChannel::Alpha) {
            mask |= metal::MTLColorWriteMaskAlpha;
        }
        self.write_mask = mask;
        self
    }

    fn set_blending(&mut self, v: bool) -> &mut dyn base::RasterizerColorTarget {
        self.blending = v;
        self
    }

    fn set_src_alpha_factor(&mut self, v: base::BlendFactor) -> &mut dyn base::RasterizerColorTarget {
        self.src_alpha_factor = translate_blend_factor(v);
        self
    }

    fn set_src_rgb_factor(&mut self, v: base::BlendFactor) -> &mut dyn base::RasterizerColorTarget {
        self.src_rgb_factor = translate_blend_factor(v);
        self
    }

    fn set_dst_alpha_factor(&mut self, v: base::BlendFactor) -> &mut dyn base::RasterizerColorTarget {
        self.dst_alpha_factor = translate_blend_factor(v);
        self
    }

    fn set_dst_rgb_factor(&mut self, v: base::BlendFactor) -> &mut dyn base::RasterizerColorTarget {
        self.dst_rgb_factor = translate_blend_factor(v);
        self
    }

    fn set_alpha_op(&mut self, v: base::BlendOp) -> &mut dyn base::RasterizerColorTarget {
        self.alpha_op = translate_blend_op(v);
        self
    }

    fn set_rgb_op(&mut self, v: base::BlendOp) -> &mut dyn base::RasterizerColorTarget {
        self.rgb_op = translate_blend_op(v);
        self
    }
}

fn translate_blend_factor(value: base::BlendFactor) -> metal::MTLBlendFactor {
    match value {
        base::BlendFactor::Zero => metal::MTLBlendFactor::Zero,
        base::BlendFactor::One => metal::MTLBlendFactor::One,
        base::BlendFactor::SrcColor => metal::MTLBlendFactor::SourceColor,
        base::BlendFactor::OneMinusSrcColor => metal::MTLBlendFactor::OneMinusSourceColor,
        base::BlendFactor::SrcAlpha => metal::MTLBlendFactor::SourceAlpha,
        base::BlendFactor::OneMinusSrcAlpha => metal::MTLBlendFactor::OneMinusSourceAlpha,
        base::BlendFactor::DstColor => metal::MTLBlendFactor::DestinationColor,
        base::BlendFactor::OneMinusDstColor => metal::MTLBlendFactor::OneMinusDestinationColor,
        base::BlendFactor::DstAlpha => metal::MTLBlendFactor::DestinationAlpha,
        base::BlendFactor::OneMinusDstAlpha => metal::MTLBlendFactor::OneMinusDestinationAlpha,
        base::BlendFactor::ConstantColor => metal::MTLBlendFactor::BlendColor,
        base::BlendFactor::OneMinusConstantColor => metal::MTLBlendFactor::OneMinusBlendColor,
        base::BlendFactor::ConstantAlpha => metal::MTLBlendFactor::BlendAlpha,
        base::BlendFactor::OneMinusConstantAlpha => metal::MTLBlendFactor::OneMinusBlendAlpha,
        base::BlendFactor::SrcAlphaSaturated => metal::MTLBlendFactor::SourceAlphaSaturated,
        base::BlendFactor::Src1Color => metal::MTLBlendFactor::Source1Color,
        base::BlendFactor::OneMinusSrc1Color => metal::MTLBlendFactor::OneMinusSource1Color,
        base::BlendFactor::Src1Alpha => metal::MTLBlendFactor::Source1Alpha,
        base::BlendFactor::OneMinusSrc1Alpha => metal::MTLBlendFactor::OneMinusSource1Alpha,
    }
}

fn translate_blend_op(value: base::BlendOp) -> metal::MTLBlendOperation {
    match value {
        base::BlendOp::Add => metal::MTLBlendOperation::Add,
        base::BlendOp::Subtract => metal::MTLBlendOperation::Subtract,
        base::BlendOp::ReverseSubtract => metal::MTLBlendOperation::ReverseSubtract,
        base::BlendOp::Min => metal::MTLBlendOperation::Min,
        base::BlendOp::Max => metal::MTLBlendOperation::Max,
    }
}

fn translate_stencil_op(value: base::StencilOp) -> metal::MTLStencilOperation {
    match value {
        base::StencilOp::Keep => metal::MTLStencilOperation::Keep,
        base::StencilOp::Zero => metal::MTLStencilOperation::Zero,
        base::StencilOp::Replace => metal::MTLStencilOperation::Replace,
        base::StencilOp::IncrementAndClamp => metal::MTLStencilOperation::IncrementClamp,
        base::StencilOp::DecrementAndClamp => metal::MTLStencilOperation::DecrementClamp,
        base::StencilOp::Invert => metal::MTLStencilOperation::Invert,
        base::StencilOp::IncrementAndWrap => metal::MTLStencilOperation::IncrementWrap,
        base::StencilOp::DecrementAndWrap => metal::MTLStencilOperation::DecrementWrap,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MetalStencilOps {
    stencil_failure: metal::MTLStencilOperation,
    depth_failure: metal::MTLStencilOperation,
    pass: metal::MTLStencilOperation,
    compare: metal::MTLCompareFunction,
}

impl Default for MetalStencilOps {
    fn default() -> Self {
        base::StencilOps::default().into()
    }
}

impl<'a> From<base::StencilOps> for MetalStencilOps {
    fn from(value: base::StencilOps) -> Self {
        MetalStencilOps {
            stencil_failure: translate_stencil_op(value.stencil_fail),
            depth_failure: translate_stencil_op(value.depth_fail),
            pass: translate_stencil_op(value.pass),
            compare: translate_cmp_fn(value.stencil_test),
        }
    }
}

/// Implementation of `RenderPipeline` for Metal.
#[derive(Debug, Clone)]
pub struct RenderPipeline {
    data: Arc<RenderPipelineData>,
}

zangfx_impl_handle! { RenderPipeline, base::RenderPipelineRef }

#[derive(Debug)]
struct RenderPipelineData {
    metal_pipeline: OCPtr<metal::MTLRenderPipelineState>,
    rast_partial_states: Option<RasterizerPartialStates>,
    prim_type: metal::MTLPrimitiveType,
    vb_start_index: u32,
    vb_used: u32,
}

unsafe impl Send for RenderPipelineData {}
unsafe impl Sync for RenderPipelineData {}

impl RenderPipeline {
    pub fn metal_pipeline(&self) -> metal::MTLRenderPipelineState {
        *self.data.metal_pipeline
    }
}

/// A compact representation of depth and stencil states. `RenderStateManager`
/// uses this to see if setting a new `MTLDepthStencilState` is necessary.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct CompactDsState {
    /// - `[0]` = depth write
    /// - `[3:1]` = depth test
    /// - `[6:4]` = front stencil test
    /// - `[9:7]` = front stencil op: stencil fail
    /// - `[12:10]` = front stencil op: depth fail
    /// - `[15:13]` = front stencil op: pass
    /// - `[22:20]` = back stencil test
    /// - `[25:23]` = back stencil op: stencil fail
    /// - `[28:26]` = back stencil op: depth fail
    /// - `[31:29]` = back stencil op: pass
    flags: u32,
    stencil_masks: [base::StencilMasks; 2],
}

impl CompactDsState {
    fn invalid() -> Self {
        Self {
            flags: !0u32,
            stencil_masks: Default::default(),
        }
    }

    fn from_rasterizer(rasterizer: &Rasterizer) -> Self {
        let flags = if rasterizer.depth_write { 1u32 } else { 0u32 }
            | ((rasterizer.depth_test as u32) << 1)
            | ((rasterizer.stencil_ops[0].compare as u32) << 4)
            | ((rasterizer.stencil_ops[0].stencil_failure as u32) << 7)
            | ((rasterizer.stencil_ops[0].depth_failure as u32) << 10)
            | ((rasterizer.stencil_ops[0].pass as u32) << 13)
            | ((rasterizer.stencil_ops[1].compare as u32) << 20)
            | ((rasterizer.stencil_ops[1].stencil_failure as u32) << 23)
            | ((rasterizer.stencil_ops[1].depth_failure as u32) << 26)
            | ((rasterizer.stencil_ops[1].pass as u32) << 29);
        Self {
            flags,
            stencil_masks: rasterizer.stencil_masks,
        }
    }
}

/// Maintains a set of render states of a render command encoder.
#[derive(Debug)]
pub(crate) struct RenderStateManager {
    metal_encoder: metal::MTLRenderCommandEncoder,
    extents: [u32; 2],
    scissor: metal::MTLScissorRect,
    cull_mode: metal::MTLCullMode,
    front_face: metal::MTLWinding,
    depth_clip_mode: metal::MTLDepthClipMode,
    triangle_fill_mode: metal::MTLTriangleFillMode,
    compact_depth_stencil: CompactDsState,

    primitive_type: metal::MTLPrimitiveType,

    index_buffer: metal::MTLBuffer,
    index_offset: base::DeviceSize,
    index_format: metal::MTLIndexType,

    vb_start_index: u32,
    vb_buffers: [metal::MTLBuffer; crate::MAX_NUM_VERTEX_BUFFERS],
    vb_offsets: [base::DeviceSize; crate::MAX_NUM_VERTEX_BUFFERS],
    vb_dirty: u32,
    vb_used: u32,
}

impl RenderStateManager {
    /// Construct a `RenderStateManager`.
    ///
    /// Ir's up to the caller to maintain the lifetime of `metal_encoder`.
    /// The render state are assumed to have the default values.
    crate unsafe fn new(metal_encoder: metal::MTLRenderCommandEncoder, extents: [u32; 2]) -> Self {
        Self {
            metal_encoder,
            extents,
            // Default values of `MTLRenderCommandEncoder` (not ZanGFX's!)
            scissor: metal::MTLScissorRect {
                x: 0,
                y: 0,
                width: extents[0] as u64,
                height: extents[1] as u64,
            },
            cull_mode: metal::MTLCullMode::None,
            front_face: metal::MTLWinding::CounterClockwise,
            depth_clip_mode: metal::MTLDepthClipMode::Clip,
            triangle_fill_mode: metal::MTLTriangleFillMode::Fill,
            compact_depth_stencil: CompactDsState::invalid(),

            // On Metal, primitive type must be specified via draw commands
            primitive_type: metal::MTLPrimitiveType::Triangle,

            index_buffer: metal::MTLBuffer::nil(),
            index_offset: 0,
            index_format: metal::MTLIndexType::UInt16,

            vb_start_index: 0,
            vb_buffers: [metal::MTLBuffer::nil(); crate::MAX_NUM_VERTEX_BUFFERS],
            vb_offsets: [0; crate::MAX_NUM_VERTEX_BUFFERS],
            vb_dirty: !0u32,
            vb_used: 0,
        }
    }

    crate fn bind_pipeline(&mut self, pipeline: &base::RenderPipelineRef) {
        let pipeline: &RenderPipeline = pipeline.downcast_ref().expect("bad render pipeline type");

        self.metal_encoder
            .set_render_pipeline_state(*pipeline.data.metal_pipeline);

        if let Some(ref rps) = pipeline.data.rast_partial_states {
            if let Some(scissor) = rps.scissor {
                self.set_scissors(0, &[scissor]);
            }
            if self.cull_mode != rps.cull_mode {
                self.metal_encoder.set_cull_mode(rps.cull_mode);
                self.cull_mode = rps.cull_mode;
            }
            if self.front_face != rps.front_face {
                self.metal_encoder.set_front_facing_winding(rps.front_face);
                self.front_face = rps.front_face;
            }
            if self.depth_clip_mode != rps.depth_clip_mode {
                self.metal_encoder.set_depth_clip_mode(rps.depth_clip_mode);
                self.depth_clip_mode = rps.depth_clip_mode;
            }
            if self.triangle_fill_mode != rps.triangle_fill_mode {
                self.metal_encoder
                    .set_triangle_fill_mode(rps.triangle_fill_mode);
                self.triangle_fill_mode = rps.triangle_fill_mode;
            }
            if let Some(depth_bias) = rps.depth_bias {
                self.set_depth_bias(Some(depth_bias));
            }
            if self.compact_depth_stencil != rps.compact_depth_stencil {
                self.metal_encoder
                    .set_depth_stencil_state(*rps.depth_stencil);
                self.compact_depth_stencil = rps.compact_depth_stencil;
            }
        }

        if self.vb_start_index != pipeline.data.vb_start_index {
            self.vb_start_index = pipeline.data.vb_start_index;
            self.vb_dirty = !0u32;
        }

        self.primitive_type = pipeline.data.prim_type;
        self.vb_used = pipeline.data.vb_used;
    }

    crate fn set_blend_constant(&mut self, value: &[f32]) {
        self.metal_encoder
            .set_blend_color(value[0], value[1], value[2], value[3]);
    }

    crate fn set_depth_bias(&mut self, value: Option<base::DepthBias>) {
        let value = value.unwrap_or_default();
        self.metal_encoder
            .set_depth_bias(value.constant_factor, value.slope_factor, value.clamp);
    }

    crate fn set_depth_bounds(&mut self, _: Option<Range<f32>>) {
        panic!("not supported");
    }

    crate fn set_stencil_refs(&mut self, values: &[u32]) {
        if values[0] == values[1] {
            self.metal_encoder.set_stencil_reference_value(values[0]);
        } else {
            self.metal_encoder
                .set_stencil_front_back_reference_value(values[0], values[1]);
        }
    }

    crate fn set_viewports(&mut self, start_viewport: base::ViewportIndex, value: &[base::Viewport]) {
        // Multiple viewport are not supported
        if value.len() > 0 {
            debug_assert_eq!(start_viewport, 0);
            debug_assert_eq!(value.len(), 1);

            let ref value = value[0];
            self.metal_encoder.set_viewport(translate_viewport(value));
        }
    }

    crate fn set_scissors(
        &mut self,
        start_viewport: base::ViewportIndex,
        value: &[base::Rect2D<u32>],
    ) {
        // Multiple viewport are not supported
        if value.len() > 0 {
            debug_assert_eq!(start_viewport, 0);
            debug_assert_eq!(value.len(), 1);

            let ref value = value[0];
            self.metal_encoder.set_scissor_rect(clip_scissor_rect(
                &translate_scissor_rect(value),
                &self.extents,
            ));
        }
    }

    crate fn bind_arg_table(&mut self, index: base::ArgTableIndex, tables: &[(&base::ArgPoolRef, &base::ArgTableRef)]) {
        for (i, (_pool, table)) in tables.iter().enumerate() {
            let our_table: &ArgTable = table.downcast_ref().expect("bad argument table type");
            self.metal_encoder.set_vertex_buffer(
                (i + index) as u64,
                our_table.offset() as u64,
                our_table.metal_buffer(),
            );
            self.metal_encoder.set_fragment_buffer(
                (i + index) as u64,
                our_table.offset() as u64,
                our_table.metal_buffer(),
            );
        }
    }

    crate fn bind_vertex_buffers(
        &mut self,
        index: base::VertexBufferIndex,
        buffers: &[(&base::BufferRef, base::DeviceSize)],
    ) {
        for (i, &(buffer, offset)) in buffers.iter().enumerate() {
            let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
            let (metal_buffer, buffer_offset) = buffer.metal_buffer_and_offset().unwrap();
            self.vb_buffers[i + index] = metal_buffer;
            self.vb_offsets[i + index] = offset + buffer_offset;
        }
        self.vb_dirty |= <u32>::ones(index as u32..(index + buffers.len()) as u32);
    }

    fn flush_vertex_buffers(&mut self) {
        // Only update the part actually used by the current pipeline
        let update_mask = self.vb_dirty & self.vb_used;

        if update_mask == 0 {
            return;
        }

        // Merge it into a single consecutive range to minimize the number of
        // calls
        let start = update_mask.trailing_zeros();
        let end = 32 - update_mask.leading_zeros();
        self.metal_encoder.set_vertex_buffers(
            (start + self.vb_start_index) as u64,
            &self.vb_buffers[start as usize..end as usize],
            &self.vb_offsets[start as usize..end as usize],
        );

        self.vb_dirty &= !<u32>::ones(start..end);
    }

    crate fn bind_index_buffer(
        &mut self,
        buffer: &base::BufferRef,
        offset: base::DeviceSize,
        format: base::IndexFormat,
    ) {
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        let (metal_buffer, buffer_offset) = buffer.metal_buffer_and_offset().unwrap();

        // The applications must hold the reference by themselves
        self.index_buffer = metal_buffer;
        self.index_offset = offset + buffer_offset;
        self.index_format = match format {
            base::IndexFormat::U16 => metal::MTLIndexType::UInt16,
            base::IndexFormat::U32 => metal::MTLIndexType::UInt32,
        };
    }

    crate fn draw(&mut self, vertex_range: Range<u32>, instance_range: Range<u32>) {
        if vertex_range.len() == 0 {
            return;
        }
        self.flush_vertex_buffers();
        if instance_range == (0..1) {
            // FIXME: this maybe causes instance index to be undefined?
            self.metal_encoder.draw_primitives(
                self.primitive_type,
                vertex_range.start as u64,
                vertex_range.len() as u64,
            );
        } else if instance_range.len() > 0 {
            self.metal_encoder.draw_primitives_instanced(
                self.primitive_type,
                vertex_range.start as u64,
                vertex_range.len() as u64,
                instance_range.len() as u64,
                instance_range.start as u64,
            );
        }
    }

    crate fn draw_indexed(
        &mut self,
        index_buffer_range: Range<u32>,
        vertex_offset: u32,
        instance_range: Range<u32>,
    ) {
        if index_buffer_range.len() == 0 {
            return;
        }
        self.flush_vertex_buffers();
        if instance_range == (0..1) && vertex_offset == 0 {
            // FIXME: this maybe causes instance index to be undefined?
            self.metal_encoder.draw_indexed_primitives(
                self.primitive_type,
                index_buffer_range.len() as u64,
                self.index_format,
                self.index_buffer,
                self.index_offset,
            );
        } else if instance_range.len() > 0 {
            self.metal_encoder.draw_indexed_primitives_instanced(
                self.primitive_type,
                index_buffer_range.len() as u64,
                self.index_format,
                self.index_buffer,
                self.index_offset,
                instance_range.len() as u64,
                vertex_offset as i64,
                instance_range.start as u64,
            );
        }
    }

    crate fn draw_indirect(&mut self, buffer: &base::BufferRef, offset: base::DeviceSize) {
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        let (metal_buffer, buffer_offset) = buffer.metal_buffer_and_offset().unwrap();
        self.flush_vertex_buffers();
        self.metal_encoder
            .draw_indirect(self.primitive_type, metal_buffer, offset + buffer_offset);
    }

    crate fn draw_indexed_indirect(&mut self, buffer: &base::BufferRef, offset: base::DeviceSize) {
        let buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        let (metal_buffer, buffer_offset) = buffer.metal_buffer_and_offset().unwrap();
        self.flush_vertex_buffers();
        self.metal_encoder.draw_indexed_indirect(
            self.primitive_type,
            self.index_format,
            self.index_buffer,
            self.index_offset,
            metal_buffer,
            offset + buffer_offset,
        );
    }
}
