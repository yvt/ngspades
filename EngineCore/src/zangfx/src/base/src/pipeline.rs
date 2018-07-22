//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for render/compute pipeline objects.
use std::ops::Range;

use crate::arg::RootSig;
use crate::common::Rect2D;
use crate::formats::VertexFormat;
use crate::pass::RenderPass;
use crate::shader::Library;
use crate::{
    CmpFn, ColorChannelFlags, DeviceSize, RenderSubpassColorTargetIndex, SubpassIndex,
    VertexAttrIndex, VertexBufferIndex, ViewportIndex,
};
use crate::{Object, Result};

define_handle! {
    /// Render pipeline handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    RenderPipeline
}

define_handle! {
    /// Compute pipeline handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    ComputePipeline
}

/// The builder object for compute pipelines.
pub type ComputePipelineBuilder = Box<dyn ComputePipelineBuilderTrait>;

/// Trait for building compute pipelines.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, library: &Library) {
///     let pipeline = device.build_compute_pipeline()
///         .compute_shader(library, "main")
///         .build()
///         .expect("Failed to create a pipeline.");
///     # }
///
pub trait ComputePipelineBuilderTrait: Object {
    /// Set the compute shader.
    ///
    /// Mandatory.
    fn compute_shader(
        &mut self,
        library: &Library,
        entry_point: &str,
    ) -> &mut dyn ComputePipelineBuilderTrait;

    /// Set the root signature.
    ///
    /// Mandatory.
    fn root_sig(&mut self, v: &RootSig) -> &mut dyn ComputePipelineBuilderTrait;

    /// Build an `ComputePipeline`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<ComputePipeline>;
}

/// The builder object for render pipelines.
pub type RenderPipelineBuilder = Box<dyn RenderPipelineBuilderTrait>;

/// Trait for building render pipelines.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, library: Library, root_sig: RootSig,
///     #    render_pass: RenderPass) {
///     let mut builder = device.build_render_pipeline();
///
///     builder.root_sig(&root_sig)
///         .vertex_shader(&library, "vertex_kernel")
///         .fragment_shader(&library, "fragment_kernel")
///
///         .render_pass(&render_pass, 0)
///         .topology(PrimitiveTopology::TriangleStrip);
///
///     let vb_vertex: VertexBufferIndex = 0;
///     let vb_instance: VertexBufferIndex = 1;
///
///     builder.vertex_buffer(vb_vertex, 32);
///     builder.vertex_buffer(vb_instance, 4).set_rate(VertexInputRate::Instance);
///
///     builder.vertex_attr(0, vb_vertex, 0, <f32>::as_format() * 4);
///     builder.vertex_attr(1, vb_vertex, 16, <f32>::as_format() * 4);
///
///     builder.vertex_attr(2, vb_instance, 0, <u32>::as_format_unnorm() * 1);
///
///     // Enable rasterizer. Use default values for all properties.
///     builder.rasterize();
///
///     let pipeline = builder.build()
///         .expect("Failed to create a pipeline.");
///     # }
///
pub trait RenderPipelineBuilderTrait: Object {
    /// Set the vertex shader.
    ///
    /// Mandatory.
    fn vertex_shader(
        &mut self,
        library: &Library,
        entry_point: &str,
    ) -> &mut dyn RenderPipelineBuilderTrait;

    /// Set the fragment shader.
    ///
    /// Mandatory if rasterization is enabled.
    fn fragment_shader(
        &mut self,
        library: &Library,
        entry_point: &str,
    ) -> &mut dyn RenderPipelineBuilderTrait;

    /// Set the root signature.
    ///
    /// Mandatory.
    fn root_sig(&mut self, v: &RootSig) -> &mut dyn RenderPipelineBuilderTrait;

    /// Set the render pass where the render pipeline will be used.
    ///
    /// Mandatory.
    fn render_pass(
        &mut self,
        v: &RenderPass,
        subpass: SubpassIndex,
    ) -> &mut dyn RenderPipelineBuilderTrait;

    /// Define a vertex buffer binding.
    ///
    /// # Valid Usage
    ///
    ///  - `stride` must be aligned by 4 bytes.
    fn vertex_buffer(
        &mut self,
        index: VertexBufferIndex,
        stride: DeviceSize,
    ) -> &mut dyn VertexBufferBinding;

    /// Defines a vertex attribute.
    ///
    /// The return type is reserved for future extensions.
    ///
    /// # Valid Usage
    ///
    ///  - `offset` must be aligned by 4 bytes.
    ///  - `buffer` must specify a vertex buffer defined by `vertex_buffer`.
    fn vertex_attr(
        &mut self,
        index: VertexAttrIndex,
        buffer: VertexBufferIndex,
        offset: DeviceSize,
        format: VertexFormat,
    );

    /// Set the input primitive topology. Mandatory.
    fn topology(&mut self, v: PrimitiveTopology) -> &mut dyn RenderPipelineBuilderTrait;

    /// Enable rasterization.
    fn rasterize(&mut self) -> &mut dyn Rasterizer;

    /// Build an `RenderPipeline`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<RenderPipeline>;
}

/// Trait for defining a vertex buffer binding.
pub trait VertexBufferBinding: Object {
    /// Set the vertex input rate. Defaults to `Vertex`.
    fn set_rate(&mut self, rate: VertexInputRate) -> &mut dyn VertexBufferBinding;
}

/// Specifies a vertex input rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexInputRate {
    Vertex,
    Instance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveTopology {
    Points,
    Lines,
    LineStrip,
    Triangles,
    TriangleStrip,
}

/// Trait for setting the properties of the rasterization stage. All properties
/// are optional.
///
/// See also: [`RasterizerExt`].
///
/// [`RasterizerExt`]: RasterizerExt
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device) {
///     let mut builder = device.build_render_pipeline();
///
///     // Set other properties of `builder` here
///
///     {
///         let rast = builder.rasterize();
///
///         rast.set_scissors(0, &[StaticOrDynamic::Dynamic])
///             .set(CullMode::None)
///             .set(Winding::CounterClockwise); // front face
///
///         rast.color_target(0)
///             .set_blending(true)
///             .set_src_rgb_factor(BlendFactor::SrcAlpha)
///             .set_dst_rgb_factor(BlendFactor::OneMinusSrcAlpha);
///     }
///
///     let pipeline = builder.build()
///         .expect("Failed to create a pipeline.");
///     # }
///
pub trait Rasterizer: Object {
    /// Set the number of viewports.
    ///
    /// Must be less than or equal to `DeviceLimits::max_num_viewports`. Must be
    /// not zero. Defaults to `1`.
    fn set_num_viewports(&mut self, v: usize) -> &mut dyn Rasterizer;

    /// Set the scissor rect.
    ///
    /// Defaults to `Static(Rect2D::<u32>::all())`.
    fn set_scissors(
        &mut self,
        start_viewport: ViewportIndex,
        v: &[StaticOrDynamic<Rect2D<u32>>],
    ) -> &mut dyn Rasterizer;

    /// Set the cull mode. Defaults to `None`.
    fn set_cull_mode(&mut self, v: CullMode) -> &mut dyn Rasterizer;

    /// Set the front face winding. Defaults to `CounterClockwise`.
    fn set_front_face(&mut self, v: Winding) -> &mut dyn Rasterizer;

    /// Control whether fragments with depth values outside the clip volume
    /// is clipped or clamped. Defaults to `Clip`.
    fn set_depth_clip_mode(&mut self, v: DepthClipMode) -> &mut dyn Rasterizer;

    /// Set the triangle filling mode. Defaults to `Fill`.
    fn set_triangle_fill_mode(&mut self, v: TriangleFillMode) -> &mut dyn Rasterizer;

    /// Set the depth bias values. Defaults to `None`.
    fn set_depth_bias(&mut self, v: Option<StaticOrDynamic<DepthBias>>) -> &mut dyn Rasterizer;

    /// Enable the alpha-to-coverage feature. Defaults to `false`.
    fn set_alpha_to_coverage(&mut self, v: bool) -> &mut dyn Rasterizer;

    /// Specify the number of samples per pixel for MSAA targets.
    /// Defaults to `1`.
    fn set_sample_count(&mut self, v: u32) -> &mut dyn Rasterizer;

    /// Enable the depth write. Defaults to `false`.
    fn set_depth_write(&mut self, v: bool) -> &mut dyn Rasterizer;

    /// Set the depth test function. Defaults to `Always`.
    ///
    /// Specify `Always` to disable the depth test.
    fn set_depth_test(&mut self, v: CmpFn) -> &mut dyn Rasterizer;

    /// Set the stencil operations. Defaults to `Default::default()`.
    fn set_stencil_ops(&mut self, front_back: [StencilOps; 2]) -> &mut dyn Rasterizer;

    /// Set the stencil masks. Defaults to `Default::default()`.
    fn set_stencil_masks(&mut self, front_back: [StencilMasks; 2]) -> &mut dyn Rasterizer;

    /// Specify whether depth bounds tests are enabled.
    ///
    /// If `DeviceLimits::supports_depth_bounds` is `false` then `None` must be
    /// specified.
    fn set_depth_bounds(&mut self, v: Option<StaticOrDynamic<Range<f32>>>) -> &mut dyn Rasterizer;

    /// Setup the color output for a color render target at a specified index.
    ///
    /// If `DeviceLimits::supports_independent_blend` is `false` then the same
    /// property values must be supplied for all color render targets.
    fn color_target(
        &mut self,
        index: RenderSubpassColorTargetIndex,
    ) -> &mut dyn RasterizerColorTarget;
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

#[derive(Debug, Clone, Copy, Default)]
pub struct DepthBias {
    pub constant_factor: f32,
    pub slope_factor: f32,
    pub clamp: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticOrDynamic<T> {
    /// Indicates a given constant value is used for the property.
    Static(T),

    /// Indicates that the value must be specified via `RenderCmdEncoder` when
    /// encoding rendering commands.
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

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> StaticOrDynamic<U> {
        match self {
            StaticOrDynamic::Static(x) => StaticOrDynamic::Static(f(x)),
            StaticOrDynamic::Dynamic => StaticOrDynamic::Dynamic,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StencilOps {
    pub stencil_fail: StencilOp,
    pub depth_fail: StencilOp,
    pub pass: StencilOp,
    pub stencil_test: CmpFn,
}

impl ::std::default::Default for StencilOps {
    fn default() -> Self {
        Self {
            stencil_fail: StencilOp::Keep,
            depth_fail: StencilOp::Keep,
            pass: StencilOp::Keep,
            stencil_test: CmpFn::Always,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StencilOp {
    Keep,
    Zero,
    Replace,
    IncrementAndClamp,
    DecrementAndClamp,
    Invert,
    IncrementAndWrap,
    DecrementAndWrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StencilMasks {
    pub read: u32,
    pub write: u32,
}

/// Extention trait for `Rasterizer`.
pub trait RasterizerExt: Rasterizer {
    /// Set a property of `Rasterizer`. The property is determined
    /// from the type of the value.
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::*;
    ///     # fn test(rasterizer: &mut Rasterizer) {
    ///     rasterizer
    ///         .set(CullMode::Back)
    ///         .set(TriangleFillMode::Line);
    ///     # }
    fn set<T: RasterizerProp>(&mut self, value: T) -> &mut Self {
        value.assign(self);
        self
    }
}

impl<T: ?Sized + Rasterizer> RasterizerExt for T {}

/// Property value that can be assigned to a property of`Rasterizer`.
pub trait RasterizerProp {
    fn assign<T: Rasterizer + ?Sized>(&self, o: &mut T);
}

impl RasterizerProp for Winding {
    fn assign<T: Rasterizer + ?Sized>(&self, o: &mut T) {
        o.set_front_face(*self);
    }
}

impl RasterizerProp for CullMode {
    fn assign<T: Rasterizer + ?Sized>(&self, o: &mut T) {
        o.set_cull_mode(*self);
    }
}

impl RasterizerProp for DepthClipMode {
    fn assign<T: Rasterizer + ?Sized>(&self, o: &mut T) {
        o.set_depth_clip_mode(*self);
    }
}

impl RasterizerProp for TriangleFillMode {
    fn assign<T: Rasterizer + ?Sized>(&self, o: &mut T) {
        o.set_triangle_fill_mode(*self);
    }
}

impl RasterizerProp for DepthBias {
    fn assign<T: Rasterizer + ?Sized>(&self, o: &mut T) {
        o.set_depth_bias(Some(StaticOrDynamic::Static(*self)));
    }
}

/// Trait for setting the properties of the color output stage of a render
/// pipeline.
pub trait RasterizerColorTarget: Object {
    /// Set the write mask. Defaults to `ColorChannel::all()`.
    fn set_write_mask(&mut self, v: ColorChannelFlags) -> &mut dyn RasterizerColorTarget;

    /// Enable blending. Defaults to `false`.
    fn set_blending(&mut self, v: bool) -> &mut dyn RasterizerColorTarget;

    /// Set the source blend factor for the alpha channel. Defaults to `One`.
    fn set_src_alpha_factor(&mut self, v: BlendFactor) -> &mut dyn RasterizerColorTarget;

    /// Set the source blend factor for RGB channels. Defaults to `One`.
    fn set_src_rgb_factor(&mut self, v: BlendFactor) -> &mut dyn RasterizerColorTarget;

    /// Set the destination blend factor for the alpha channel. Defaults to `Zero`.
    fn set_dst_alpha_factor(&mut self, v: BlendFactor) -> &mut dyn RasterizerColorTarget;

    /// Set the destination blend factor for RGB channels. Defaults to `Zero`.
    fn set_dst_rgb_factor(&mut self, v: BlendFactor) -> &mut dyn RasterizerColorTarget;

    /// Set the blending operation for the alpha channel. Defaults to `Add`.
    fn set_alpha_op(&mut self, v: BlendOp) -> &mut dyn RasterizerColorTarget;

    /// Set the blending operation for RGB channels. Defaults to `Add`.
    fn set_rgb_op(&mut self, v: BlendOp) -> &mut dyn RasterizerColorTarget;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendOp {
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
    SrcColor,
    OneMinusSrcColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstColor,
    OneMinusDstColor,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturated,

    // the second color output
    Src1Color,
    OneMinusSrc1Color,
    Src1Alpha,
    OneMinusSrc1Alpha,
}
