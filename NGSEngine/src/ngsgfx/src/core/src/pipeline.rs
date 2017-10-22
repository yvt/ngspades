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

use ngsenumflags::BitFlags;
use cgmath::Vector2;

use {VertexBindingLocation, VertexAttributeLocation, VertexFormat, RenderPass, Rect2D,
     CompareFunction, PipelineLayout, ShaderStage, ShaderModule, SubpassIndex, Validate,
     DeviceCapabilities};

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
    pub subpass_index: SubpassIndex,

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

    /// Scissor rect.
    ///
    /// All coordinate values must lie in the range `[0, i32::max_value()]`.
    pub scissor_rect: StaticOrDynamic<Rect2D<u32>>,

    // rasterization state
    pub cull_mode: CullMode,
    pub front_face: Winding,

    /// Controls whether fragments with depth values outside the clip volume
    /// is clipped or clamped.
    pub depth_clip_mode: DepthClipMode,
    pub triangle_fill_mode: TriangleFillMode,
    pub depth_bias: StaticOrDynamic<Option<DepthBias>>,

    // multisample state
    pub alpha_to_coverage: bool,
    pub sample_count: u32,

    // depth stencil state
    pub depth_write: bool,
    pub depth_test: CompareFunction,
    pub stencil_ops: [StencilOperations; 2],
    pub stencil_masks: StaticOrDynamic<[StencilMasks; 2]>,
    pub stencil_references: StaticOrDynamic<[u32; 2]>,

    /// Specifies whether depth bounds tests are enabled.
    ///
    /// If `DeviceLimits::supports_depth_bounds` is `false` then `None` must be
    /// specified.
    pub depth_bounds: Option<StaticOrDynamic<DepthBounds>>,

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
                Vector2::new(i32::max_value() as u32, i32::max_value() as u32),
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
            stencil_ops: Default::default(),
            stencil_masks: StaticOrDynamic::Static(Default::default()),
            stencil_references: StaticOrDynamic::Static([0, 0]),
            depth_bounds: None,
            blend_constants: StaticOrDynamic::Static([0f32; 4]),
            color_attachments: &[],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StencilOperations {
    pub stencil_fail_operation: StencilOperation,
    pub depth_fail_operation: StencilOperation,
    pub pass_operation: StencilOperation,
    pub compare_function: CompareFunction,
}

impl ::std::default::Default for StencilOperations {
    fn default() -> Self {
        Self {
            stencil_fail_operation: StencilOperation::Keep,
            depth_fail_operation: StencilOperation::Keep,
            pass_operation: StencilOperation::Keep,
            compare_function: CompareFunction::Always,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StencilStateDescription<'a, TGraphicsPipeline: GraphicsPipeline> {
    /// Specifies `GraphicsPipeline` the `StencilState` is based on.
    ///
    /// The specified `GraphicsPipeline` must have been created with
    /// `GraphicsPipelineRasterizerDescription::stencil_masks` set to `Dynamic`.
    pub pipeline: &'a TGraphicsPipeline,

    pub masks: [StencilMasks; 2],

    pub label: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct StencilMasks {
    pub read_mask: u32,
    pub write_mask: u32,
}

impl ::std::default::Default for StencilMasks {
    fn default() -> Self {
        Self {
            read_mask: 0,
            write_mask: 0,
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

impl<T> StaticOrDynamic<T> {
    pub fn as_ref(&self) -> StaticOrDynamic<&T> {
        match self {
            &StaticOrDynamic::Static(ref x) => StaticOrDynamic::Static(x),
            &StaticOrDynamic::Dynamic => StaticOrDynamic::Dynamic,
        }
    }

    pub fn as_mut(&mut self) -> StaticOrDynamic<&mut T> {
        match self {
            &mut StaticOrDynamic::Static(ref mut x) => StaticOrDynamic::Static(x),
            &mut StaticOrDynamic::Dynamic => StaticOrDynamic::Dynamic,
        }
    }

    pub fn static_value(self) -> Option<T> {
        match self {
            StaticOrDynamic::Static(x) => Some(x),
            StaticOrDynamic::Dynamic => None,
        }
    }

    pub fn is_static(&self) -> bool {
        match self {
            &StaticOrDynamic::Static(_) => true,
            &StaticOrDynamic::Dynamic => false,
        }
    }

    pub fn is_dynamic(&self) -> bool {
        match self {
            &StaticOrDynamic::Static(_) => false,
            &StaticOrDynamic::Dynamic => true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VertexBufferLayoutDescription {
    pub binding: VertexBindingLocation,
    /// Vertex stride in bytes.
    ///
    /// Must be a multiple of 4 bytes.
    pub stride: u32,
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
    pub offset: u32,
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
    /// Polygons are rasterized by drawing edges instead of filling the
    /// inside of them.
    ///
    /// Requires a non-solid fill mode feature and
    /// `DeviceLimits::supports_fill_mode_non_solid` indicates whether it is
    /// supported by the device.
    Line,

    /// Polygons are rasterized by filling the inside of them.
    Fill,
}

/// Controls whether fragments with depth values outside the clip volume
/// is clipped or clamped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepthClipMode {
    /// Fragments with depth values outside the clip volume are clipped.
    Clip,

    /// Fragments with depth values outside the clip volume are not clipped
    /// and the depth values are clamped.
    ///
    /// Requires a depth clamping feature and `DeviceLimits::supports_depth_clamp`
    /// indicates whether it is supported by the device.
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

#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash)]
#[repr(u32)]
pub enum ColorWriteMask {
    Red = 0b0001,
    Green = 0b0010,
    Blue = 0b0100,
    Alpha = 0b1000,
}

impl ColorWriteMask {
    pub fn all() -> BitFlags<ColorWriteMask> {
        ColorWriteMask::Red | ColorWriteMask::Green | ColorWriteMask::Blue |
            ColorWriteMask::Alpha
    }
}

/// Validation errors for [`ComputePipelineDescription`](struct.ComputePipelineDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ComputePipelineDescriptionValidationError {
    // TODO
}

impl<'a, TPipelineLayout: PipelineLayout, TShaderModule: ShaderModule> Validate
    for ComputePipelineDescription<'a, TPipelineLayout, TShaderModule> {
    type Error = ComputePipelineDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        // TODO
    }
}

/// Validation errors for [`GraphicsPipelineDescription`](struct.GraphicsPipelineDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum GraphicsPipelineDescriptionValidationError {
    // TODO
}

impl<
    'a,
    TRenderPass: RenderPass,
    TPipelineLayout: PipelineLayout,
    TShaderModule: ShaderModule,
> Validate for GraphicsPipelineDescription<'a, TRenderPass, TPipelineLayout, TShaderModule> {
    type Error = GraphicsPipelineDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        // TODO
    }
}

/// Validation errors for [`StencilStateDescription`](struct.StencilStateDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum StencilStateDescriptionValidationError {
    // TODO
}

impl<'a, TGraphicsPipeline: GraphicsPipeline> Validate
    for StencilStateDescription<'a, TGraphicsPipeline> {
    type Error = StencilStateDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        // TODO
    }
}
