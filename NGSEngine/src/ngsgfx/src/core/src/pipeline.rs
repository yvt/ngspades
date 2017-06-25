//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::clone::Clone;
use std::hash::Hash;
use std::fmt::Debug;
use std::cmp::{Eq, PartialEq};
use std::any::Any;
use std::i32;

use enumflags::BitFlags;
use cgmath::Vector2;

use super::{VertexBindingLocation, VertexAttributeLocation, VertexFormat, RenderPass, Rect2D,
            CompareFunction, PipelineLayout, ShaderStage, ShaderModule, Marker};

/// Handle for compute pipeline objects.
pub trait ComputePipeline
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {
}

/// Handle for graphics pipeline objects.
pub trait GraphicsPipeline
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {
}

/// Handle for stencil state objects.
pub trait StencilState
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {
}

#[derive(Debug, Clone, Copy)]
pub struct ComputePipelineDescription<'a, TPipelineLayout: PipelineLayout, TShaderModule: ShaderModule> {
    pub shader_stage: ShaderStageDescription<'a, TShaderModule>,
    pub pipeline_layout: &'a TPipelineLayout,

    pub label: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicsPipelineDescription<'a, TRenderPass: RenderPass, TPipelineLayout: PipelineLayout, TShaderModule: ShaderModule> {
    /// Shader stages.
    ///
    /// The same type of shader stage shall never appear twice.
    /// Must contain a vertex shader stage.
    /// May contain a fragment shader stage if and only if
    /// `rasterizer` is not `None`.
    pub shader_stages: &'a [ShaderStageDescription<'a, TShaderModule>],

    // input assembler
    pub vertex_buffers: &'a [VertexBufferLayoutDescription],
    pub vertex_attributes: &'a [VertexAttributeDescription],

    // vertex input
    pub topology: PrimitiveTopology,
    // primitive restart is always enabled

    // tesselation is intentionally excluded because I don't care about that
    pub rasterizer: Option<GraphicsPipelineRasterizerDescription<'a>>,

    pub pipeline_layout: &'a TPipelineLayout,
    pub render_pass: &'a TRenderPass,
    pub subpass_index: usize,

    pub label: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ShaderStageDescription<'a, TShaderModule: ShaderModule> {
    /// Names a single pipeline stage.
    pub stage: ShaderStage,
    /// Specifies the shader module containing the shader for this stage.
    pub module: &'a TShaderModule,
    /// Specifies the entry point name of the shader.
    pub entry_point_name: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicsPipelineRasterizerDescription<'a> {
    // viewport/scissor
    pub viewport: StaticOrDynamic<Viewport>,
    pub scissor_rect: StaticOrDynamic<Rect2D<u32>>,

    // rasterization state
    pub cull_mode: CullMode,
    pub front_face: Winding,
    pub depth_clip_mode: DepthClipMode,
    pub triangle_fill_mode: TriangleFillMode,
    pub depth_bias: StaticOrDynamic<Option<DepthBias>>,

    // multisample state
    pub alpha_to_coverage: bool,
    pub sample_count: u32,

    // depth stencil state
    pub depth_write: bool,
    pub depth_test: CompareFunction,
    pub stencil: StaticOrDynamic<StencilDescriptionSet>,
    pub depth_bounds: StaticOrDynamic<Option<DepthBounds>>,

    // color blend state
    pub blend_constants: StaticOrDynamic<[f32; 4]>,
    pub color_attachments: &'a [GraphicsPipelineColorAttachmentDescription],
}

impl<'a> ::std::default::Default for GraphicsPipelineRasterizerDescription<'a> {
    fn default() -> Self {
        Self {
            viewport: StaticOrDynamic::Dynamic,
            scissor_rect: StaticOrDynamic::Static(Rect2D::new(
                Vector2::new(0, 0),
                Vector2::new(u32::max_value(), u32::max_value()),
            )),
            cull_mode: CullMode::Back,
            front_face: Winding::CounterClockwise,
            depth_clip_mode: DepthClipMode::Clip,
            triangle_fill_mode: TriangleFillMode::Fill,
            depth_bias: StaticOrDynamic::Static(None),
            alpha_to_coverage: false,
            sample_count: 1,
            depth_write: true,
            depth_test: CompareFunction::LessEqual,
            stencil: StaticOrDynamic::Static(Default::default()),
            depth_bounds: StaticOrDynamic::Static(None),
            blend_constants: StaticOrDynamic::Static([0f32; 4]),
            color_attachments: &[],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StencilStateDescription<'a, TGraphicsPipeline: GraphicsPipeline> {
    pub pipeline: &'a TGraphicsPipeline,
    pub set: StencilDescriptionSet,

    pub label: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StencilDescriptionSet {
    pub front: StencilDescription,
    pub back: StencilDescription,
}

#[derive(Debug, Clone, Copy)]
pub struct StencilDescription {
    pub stencil_fail_operation: StencilOperation,
    pub depth_fail_operation: StencilOperation,
    pub pass_operation: StencilOperation,
    pub compare_function: CompareFunction,
    pub read_mask: u32,
    pub write_mask: u32,
    pub reference: u32,
}

impl ::std::default::Default for StencilDescription {
    fn default() -> Self {
        Self {
            stencil_fail_operation: StencilOperation::Keep,
            depth_fail_operation: StencilOperation::Keep,
            pass_operation: StencilOperation::Keep,
            compare_function: CompareFunction::Never,
            read_mask: 0,
            write_mask: 0,
            reference: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StencilOperation {
    Keep,
    Zero,
    Replace,
    IncrementAndClamp,
    DecrementAndClamp,
    Invert,
    IncrementAndWrap,
    DecrementAndWrap,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicsPipelineColorAttachmentDescription {
    pub blending: Option<BlendStateDescription>,
    pub write_mask: BitFlags<ColorWriteMask>,
}

impl ::std::default::Default for GraphicsPipelineColorAttachmentDescription {
    fn default() -> Self {
        Self {
            blending: None,
            write_mask: ColorWriteMask::all(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlendStateDescription {
    pub source_alpha_factor: BlendFactor,
    pub source_rgb_factor: BlendFactor,
    pub destination_alpha_factor: BlendFactor,
    pub destination_rgb_factor: BlendFactor,
    pub rgb_blend_operation: BlendOperation,
    pub alpha_blend_operation: BlendOperation,
}

impl ::std::default::Default for BlendStateDescription {
    fn default() -> Self {
        Self {
            source_alpha_factor: BlendFactor::One,
            source_rgb_factor: BlendFactor::One,
            destination_alpha_factor: BlendFactor::Zero,
            destination_rgb_factor: BlendFactor::Zero,
            rgb_blend_operation: BlendOperation::Add,
            alpha_blend_operation: BlendOperation::Add,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendOperation {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendFactor {
    Zero,
    One,
    SourceColor,
    OneMinusSourceColor,
    SourceAlpha,
    OneMinusSourceAlpha,
    DestinationColor,
    OneMinusDestinationColor,
    DestinationAlpha,
    OneMinusDestinationAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SourceAlphaSaturated,

    // the second color output
    Source1Color,
    OneMinusSource1Color,
    Source1Alpha,
    OneMinusSource1Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticOrDynamic<T> {
    Static(T),
    Dynamic,
}

#[derive(Debug, Clone, Copy)]
pub struct VertexBufferLayoutDescription {
    pub binding: VertexBindingLocation,
    /// Vertex stride in bytes.
    ///
    /// Must be a multiple of 4 bytes.
    pub stride: usize,
    pub input_rate: VertexInputRate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexInputRate {
    Vertex,
    Instance,
}

#[derive(Debug, Clone, Copy)]
pub struct VertexAttributeDescription {
    pub location: VertexAttributeLocation,
    pub binding: VertexBindingLocation,
    pub format: VertexFormat,
    /// The location of the vertex data in bytes.
    ///
    /// Must be a multiple of 4 bytes.
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveTopology {
    Points,
    Lines,
    LineStrip,
    Triangles,
    TriangleStrip,
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    /// The X coordinate of the viewport's upper left corner.
    pub x: f32,
    /// The Y coordinate of the viewport's upper left corner.
    pub y: f32,
    /// The width of the viewport (measure in pixels).
    pub width: f32,
    /// The height of the viewport (measure in pixels).
    pub height: f32,
    /// The lower bound of the viewport's depth range.
    pub min_depth: f32,
    /// The upper bound of the viewport's depth range.
    pub max_depth: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriangleFillMode {
    Line,
    Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepthClipMode {
    Clip,
    Clamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Winding {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CullMode {
    /// Specifies that no primitives are culled.
    None,
    /// Specifies that front-facing primitives are culled.
    Front,
    /// Specifies that back-facing primitives are culled.
    Back,
}

#[derive(Debug, Clone, Copy)]
pub struct DepthBias {
    pub constant_factor: f32,
    pub slope_factor: f32,
    pub clamp: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct DepthBounds {
    pub min: f32,
    pub max: f32,
}

// prevent `InnerXXX` from being exported
mod flags {
    #[derive(EnumFlags, Copy, Clone, Debug, Hash)]
    #[repr(u32)]
    pub enum ColorWriteMask {
        Red = 0b0001,
        Green = 0b0010,
        Blue = 0b0100,
        Alpha = 0b1000,
    }

    impl ColorWriteMask {
        pub fn all() -> super::BitFlags<ColorWriteMask> {
            ColorWriteMask::Red | ColorWriteMask::Green | ColorWriteMask::Blue |
                ColorWriteMask::Alpha
        }
    }
}

pub use self::flags::ColorWriteMask;

// TODO: validation of pipeline descriptions
