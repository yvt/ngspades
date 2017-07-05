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

use {RenderPass, ImageView, Validate, DeviceCapabilities, Marker};

/// Handle for framebuffer objects.
pub trait Framebuffer
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any + Marker {
}

#[derive(Debug, Clone, Copy)]
pub struct FramebufferDescription<'a, TRenderPass: RenderPass, TImageView: ImageView> {
    pub render_pass: &'a TRenderPass,
    pub attachments: &'a [FramebufferAttachmentDescription<'a, TImageView>],
    pub width: u32,
    pub height: u32,

    /// Must be `1`.
    pub num_layers: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct FramebufferAttachmentDescription<'a, TImageView: ImageView> {
    /// Specified the image view to use as the corresponding attachment.
    ///
    /// - Must only specify a single mipmap level.
    /// - Must have a single array layer.
    /// - Must be at least as large as the dimensions specified in
    ///   `FramebufferDescription`.
    pub image_view: &'a TImageView,
    pub clear_values: ClearValues,
}

#[derive(Debug, Clone, Copy)]
pub enum ClearValues {
    /// Clear values for a color attachment with a format other than unnormalized integer ones.
    ColorFloat([f32; 4]),

    /// Clear values for a color attachment with an unnormalized unsigned integer format.
    ColorUnsignedInteger([u32; 4]),

    /// Clear values for a color attachment with an unnormalized signed integer format.
    ColorSignedInteger([i32; 4]),

    /// Clear values for depth and stencil attachments.
    DepthStencil(f32, u32),
}

/// Validation errors for [`FramebufferDescription`](struct.FramebufferDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum FramebufferDescriptionValidationError {
    // TODO
}

impl<'a, TRenderPass: RenderPass, TImageView: ImageView> Validate
    for FramebufferDescription<'a, TRenderPass, TImageView> {
    type Error = FramebufferDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        // TODO
    }
}
