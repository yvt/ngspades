//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for render pass objects and render target objects, and other
//! relevant types.
use crate::formats::ImageFormat;
use crate::resources::ImageRef;
use crate::AccessTypeFlags;
use crate::{Object, Result};
use crate::{RenderPassTargetIndex, SubpassIndex};

define_handle! {
    /// Render pass handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    RenderPassRef
}

define_handle! {
    /// Render target table handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    RenderTargetTableRef
}

/// The builder object for render passes.
pub type RenderPassBuilderRef = Box<dyn RenderPassBuilder>;

/// Trait for building render passes.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::formats::ImageFormat;
///     # use zangfx_base::pass::{RenderPassBuilder, StoreOp};
///     # fn test(device: &Device) {
///     let mut builder = device.build_render_pass();
///
///     builder.target(0)
///         .set_format(ImageFormat::SrgbBgra8)
///         .set_store_op(StoreOp::Store);
///     builder.target(1)
///         .set_format(ImageFormat::DepthFloat32);
///
///     // Subpass #0
///     builder.subpass_color_targets(&[Some(0)]);
///     builder.subpass_ds_target(Some(1));
///
///     let render_pass = builder.build()
///         .expect("Failed to create a render pass.");
///     # }
///
pub trait RenderPassBuilder: Object {
    /// Define a render target of the render pass.
    ///
    /// Use the returned `dyn RenderPassTarget` to specify additional properties
    /// (some of which are mandatory) of it.
    ///
    /// # Valid Usage
    ///
    /// The render target index must be assigned densely, starting from zero.
    fn target(&mut self, index: RenderPassTargetIndex) -> &mut dyn RenderPassTarget;

    /// Define a subpass dependency between one of the previous subpasses and
    /// the current one.
    ///
    /// `from` specifies the source subpass index. If `from` is equal to the
    /// current subpass index, it defines a subpass self-dependency, which is
    /// required to use the [`barrier`] command inside the subpass.
    ///
    /// [`barrier`]: crate::CmdEncoderExt::barrier
    ///
    /// # Valid Usage
    ///
    ///  - `from` shall be less than or equal to the current subpass index.
    fn subpass_dep(
        &mut self,
        from: SubpassIndex,
        src_access: AccessTypeFlags,
        dst_access: AccessTypeFlags,
    ) -> &mut dyn RenderPassBuilder;

    /// Define the color render targets of the current subpass.
    ///
    /// The return type of this method is reserved for future extensions.
    ///
    /// # Valid Usage
    ///
    /// You must specify at least one non-`None` color target.
    fn subpass_color_targets(&mut self, targets: &[Option<RenderPassTargetIndex>]);

    /// Define the depth/stencil render target of the current subpass.
    ///
    /// The return type of this method is reserved for future extensions.
    fn subpass_ds_target(&mut self, target: Option<RenderPassTargetIndex>);

    // TODO: Read-only depth/stencil

    // TODO: `next_subpass`

    /// Build an `RenderPassRef`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<RenderPassRef>;
}

pub trait RenderPassTarget: Object {
    /// Set the image format for the render target.
    ///
    /// Mandatory.
    fn set_format(&mut self, v: ImageFormat) -> &mut dyn RenderPassTarget;

    /// Set the load operation for the render target.
    ///
    /// Defaults to `LoadOp::DontCare`.
    fn set_load_op(&mut self, v: LoadOp) -> &mut dyn RenderPassTarget;
    /// Set the store operation for the render target.
    ///
    /// Defaults to `StoreOp::DontCare`.
    fn set_store_op(&mut self, v: StoreOp) -> &mut dyn RenderPassTarget;

    /// Set the load operation for the stencil aspect of the render target.
    ///
    /// Defaults to `LoadOp::DontCare`.
    fn set_stencil_load_op(&mut self, v: LoadOp) -> &mut dyn RenderPassTarget;

    /// Set the store operation for the stencil aspect of the render target.
    ///
    /// Defaults to `StoreOp::DontCare`.
    fn set_stencil_store_op(&mut self, v: StoreOp) -> &mut dyn RenderPassTarget;
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

/// The builder object for render target tables.
pub type RenderTargetTableBuilderRef = Box<dyn RenderTargetTableBuilder>;

/// Trait for building render target tables.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, pass: RenderPassRef, image: ImageRef) {
///     let mut builder = device.build_render_target_table();
///     builder.render_pass(&pass)
///         .extents(&[1024, 768]);
///
///     builder.target(0, &image)
///         .clear_float(&[0.0, 0.0, 0.0, 1.0]);
///
///     let rt_table = builder.build()
///         .expect("Failed to create a render target table.");
///     # }
///
pub trait RenderTargetTableBuilder: Object {
    /// Set the associated render pass to `v`.
    ///
    /// Mandatory.
    fn render_pass(&mut self, v: &RenderPassRef) -> &mut dyn RenderTargetTableBuilder;

    /// Set the render target extents to `v`.
    ///
    /// `v.len()` matches the dimensionality of the image and must be 1 or 2.
    ///
    /// Mandatory.
    fn extents(&mut self, v: &[u32]) -> &mut dyn RenderTargetTableBuilder;

    /// Set the render target layer count to `v`.
    ///
    /// Defaults to `1`.
    fn num_layers(&mut self, v: u32) -> &mut dyn RenderTargetTableBuilder;

    /// Define a render target.
    ///
    /// `image` will be attached as the render target corresponding to one at
    /// the index `index` in the render pass specified by `render_pass`.
    ///
    /// Mandatory. Must be specified for each render target defined by the
    /// render pass.
    fn target(&mut self, index: RenderPassTargetIndex, image: &ImageRef) -> &mut dyn RenderTarget;

    /// Build an `RenderTargetTableRef`.
    ///
    /// # Valid Usage
    ///
    /// - All mandatory properties must have their values set before this
    ///   method is called.
    /// - All `ImageRef`s specified via `target` must belong to a single queue.
    ///
    fn build(&mut self) -> Result<RenderTargetTableRef>;
}

pub trait RenderTarget: Object {
    /// Set the mipmap level used for rendering.
    ///
    /// Defaults to `0`.
    ///
    /// For an image view, the value is relative to the first mipmap level of
    /// the image view.
    fn mip_level(&mut self, v: u32) -> &mut dyn RenderTarget;

    /// Set the array layer used for rendering.
    ///
    /// Defaults to `0`.
    ///
    /// For an image view, the value is relative to the first array layer of
    /// the image view.
    fn layer(&mut self, v: u32) -> &mut dyn RenderTarget;

    /// Set the clear value for the render target with a format other than
    /// unnormalized integer ones.
    ///
    /// Defaults to an implementation defined value.
    /// `v.len()` must be at least `4`.
    fn clear_float(&mut self, v: &[f32]) -> &mut dyn RenderTarget;

    /// Set the clear value for the render target with an unnormalized unsigned
    /// integer format.
    ///
    /// Defaults to an implementation defined value.
    /// `v.len()` must be at least `4`.
    fn clear_uint(&mut self, v: &[u32]) -> &mut dyn RenderTarget;

    /// Set the clear value for the render target with an unnormalized signed
    /// integer format.
    ///
    /// Defaults to an implementation defined value.
    /// `v.len()` must be at least `4`.
    fn clear_sint(&mut self, v: &[i32]) -> &mut dyn RenderTarget;

    /// Set the clear value for the depth and stencil render targets.
    ///
    /// Defaults to an implementation defined value.
    fn clear_depth_stencil(&mut self, depth: f32, stencil: u32) -> &mut dyn RenderTarget;
}
