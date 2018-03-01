//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for render pass objects and render target objects, and other
//! relevant types.
use Object;

use common::Result;
use formats::ImageFormat;
use resources::ImageLayout;
use handles::{ImageView, RenderPass, RenderTargetTable};
use {RenderPassTargetIndex, SubpassIndex};
use {AccessTypeFlags, StageFlags};

/// Trait for building render passes.
///
/// # Valid Usage
///
///  - No instance of `RenderPassBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::{AccessType, Stage};
///     # use zangfx_base::formats::ImageFormat;
///     # use zangfx_base::resources::ImageLayout;
///     # use zangfx_base::pass::{RenderPassBuilder, StoreOp};
///     # fn test(device: &Device) {
///     let mut builder = device.build_render_pass();
///
///     builder.target(0)
///         .set_format(ImageFormat::SrgbBgra8)
///         .set_store_op(StoreOp::Store)
///         .set_final_layout(ImageLayout::Present);
///     builder.target(1)
///         .set_format(ImageFormat::DepthFloat32);
///
///     // Subpass #0
///     builder.subpass_color_targets(&[Some((0, ImageLayout::RenderWrite))])
///         .subpass_ds_target(Some((1, ImageLayout::RenderWrite)));
///
///     // Post-render pass external
///     builder.end()
///         .subpass_dep(
///             Some(1),
///             Stage::RenderOutput.into(),
///             AccessType::ColorWrite.into(),
///             Stage::RenderOutput.into(),
///             AccessType::ColorWrite.into(),
///         );
///
///     let render_pass = builder.build()
///         .expect("Failed to create a render pass.");
///     # }
///
pub trait RenderPassBuilder: Object {
    /// Define a render target of the render pass.
    ///
    /// Use the returned `RenderPassTarget` to specify additional properties
    /// (some of which are mandatory) of it.
    ///
    /// # Valid Usage
    ///
    /// The render target index must be assigned densely, starting from zero.
    fn target(&mut self, index: RenderPassTargetIndex) -> &mut RenderPassTarget;

    /// End the definition of subpasses. Following calls to `subpass_dep` define
    /// subpass-to-external dependencies.
    fn end(&mut self) -> &mut RenderPassBuilder;

    /// Define a subpass dependency between one of the previous subpasses and
    /// the current one.
    ///
    /// `from` specifies the source subpass index. `None` indicates an
    /// external-to-subpass dependency.
    ///
    /// External (external-to-subpass or subpass-to-external) dependencies
    /// define memory barriers required between the render pass and the set of
    /// preceding/following commands. They must be used in combination with
    /// fences. Furthermore, memory dependencies inserted with fences **must**
    /// be a subset of those expressed by subpass dependencies.
    fn subpass_dep(
        &mut self,
        from: Option<SubpassIndex>,
        src_stage: StageFlags,
        src_access: AccessTypeFlags,
        dst_stage: StageFlags,
        dst_access: AccessTypeFlags,
    ) -> &mut RenderPassBuilder;

    /// Define the color render targets of the current subpass.
    fn subpass_color_targets(
        &mut self,
        targets: &[Option<(RenderPassTargetIndex, ImageLayout)>],
    ) -> &mut RenderPassBuilder;

    /// Define the depth/stencil render target of the current subpass.
    fn subpass_ds_target(
        &mut self,
        target: Option<(RenderPassTargetIndex, ImageLayout)>,
    ) -> &mut RenderPassBuilder;

    // TODO: `next_subpass`

    /// Build an `RenderPass`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<RenderPass>;
}

pub trait RenderPassTarget: Object {
    /// Set the image format for the render target.
    ///
    /// Mandatory.
    fn set_format(&mut self, v: ImageFormat) -> &mut RenderPassTarget;

    /// Set the load operation for the render target.
    ///
    /// Defaults to `LoadOp::DontCare`.
    fn set_load_op(&mut self, v: LoadOp) -> &mut RenderPassTarget;
    /// Set the store operation for the render target.
    ///
    /// Defaults to `StoreOp::DontCare`.
    fn set_store_op(&mut self, v: StoreOp) -> &mut RenderPassTarget;

    /// Set the load operation for the stencil aspect of the render target.
    ///
    /// Defaults to `LoadOp::DontCare`.
    fn set_stencil_load_op(&mut self, v: LoadOp) -> &mut RenderPassTarget;

    /// Set the store operation for the stencil aspect of the render target.
    ///
    /// Defaults to `StoreOp::DontCare`.
    fn set_stencil_store_op(&mut self, v: StoreOp) -> &mut RenderPassTarget;

    /// Set the initial layout for the render target.
    ///
    /// Defaults to `ImageLayout::Undefined`.
    fn set_initial_layout(&mut self, v: ImageLayout) -> &mut RenderPassTarget;
    /// Set the initial layout for the render target.
    ///
    /// Defaults to `ImageLayout::Undefined`.
    fn set_final_layout(&mut self, v: ImageLayout) -> &mut RenderPassTarget;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadOp {
    Load,
    Clear,
    DontCare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoreOp {
    Store,
    DontCare,
}

/// Trait for building render target tables.
///
/// # Valid Usage
///
///  - No instance of `RenderTargetTableBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::pass::RenderTargetTableBuilder;
///     # use zangfx_base::handles::{RenderPass, ImageView};
///     # fn test(device: &Device, pass: &RenderPass, image_view: &ImageView) {
///     let mut rt_table = device.build_rt_table()
///         .render_pass(pass)
///         .extents(&[1024, 768])
///         .target(0, image_view)
///         .build()
///         .expect("Failed to create a render target table.");
///     # }
///
pub trait RenderTargetTableBuilder: Object {
    /// Set the associated render pass to `v`.
    ///
    /// Mandatory.
    fn render_pass(&mut self, v: &RenderPass) -> &mut RenderTargetTableBuilder;

    /// Set the render target extents to `v`.
    ///
    /// `v.len()` matches the dimensionality of the image and must be 1 or 2.
    ///
    /// Mandatory.
    fn extents(&mut self, v: &[u32]) -> &mut RenderTargetTableBuilder;

    /// Set the render target layer count to `v`.
    ///
    /// Defaults to `1`.
    fn num_layers(&mut self, v: u32) -> &mut RenderTargetTableBuilder;

    /// Define a render target.
    ///
    /// `view` will be attached as the render target corresponding to one at
    /// the index `index` in the render pass specified by `render_pass`.
    ///
    /// Mandatory. Must be specified for each render target defined by the
    /// render pass.
    fn target(&mut self, index: RenderPassTargetIndex, view: &ImageView) -> &mut RenderTargetTableBuilder;

    /// Build an `ArgTableSig`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<RenderTargetTable>;
}
