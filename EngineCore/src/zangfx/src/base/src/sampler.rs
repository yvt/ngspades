//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for sampler objects, and other relevant types.
//!
//! Each device can have a limited number (which depends on the implementation;
//! usually in the order of 10s–1000s) of unique samplers created on it.
//! Samplers are never garbage-collected.
use std::ops;

use crate::{CmpFn, Object, Result};

define_handle! {
    /// Sampler handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    SamplerRef
}

/// The builder for samplers.
pub type SamplerBuilderRef = Box<dyn SamplerBuilder>;

/// Trait for building samplers.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device) {
///     let image = device.build_sampler()
///         .mag_filter(Filter::Nearest)
///         .min_filter(Filter::Nearest)
///         .lod_clamp(0.0 .. 4.0)
///         .build()
///         .expect("Failed to create a sampler.");
///     # }
///
pub trait SamplerBuilder: Object {
    /// Set the magnification filter.
    ///
    /// Defaults to `Filter::Linear`.
    fn mag_filter(&mut self, v: Filter) -> &mut dyn SamplerBuilder;

    /// Set the minification filter.
    ///
    /// Defaults to `Filter::Linear`.
    fn min_filter(&mut self, v: Filter) -> &mut dyn SamplerBuilder;

    /// Set the addressing mode for each axis of texture coordinates.
    ///
    /// When less than 3 elements are specified, the remaining ones are filled
    /// by repeating the last one. If the specified slice is empty, it is
    /// assumed to be `[AddressMode::Repeat; 3]`.
    ///
    /// Defaults to `[AddressMode::Repeat; 3]`.
    ///
    /// # Valid Usage
    ///
    ///  - The given slice must have the number of elements between 0 and 3.
    fn address_mode(&mut self, v: &[AddressMode]) -> &mut dyn SamplerBuilder;

    /// Set the mipmap interpolation mode.
    ///
    /// Defaults to `MipmapMode::Linear`.
    fn mipmap_mode(&mut self, v: MipmapMode) -> &mut dyn SamplerBuilder;

    /// Set the mipmap clamp range.
    ///
    /// Defaults to `0.0..0.0`.
    fn lod_clamp(&mut self, v: ops::Range<f32>) -> &mut dyn SamplerBuilder;

    /// Set the maximum anisotropic filtering level.
    ///
    /// Defaults to `1` (minimum).
    fn max_anisotropy(&mut self, v: u32) -> &mut dyn SamplerBuilder;

    /// Set the comparison function used when sampling from a depth texture.
    ///
    /// `Some(Never)` will be treated as `None`.
    ///
    /// Defaults to `None`.
    fn cmp_fn(&mut self, v: Option<CmpFn>) -> &mut dyn SamplerBuilder;

    /// Set the border color used for the `ClampToBorderColor` addressing mode.
    ///
    /// Defaults to `FloatTransparentBlack`.
    fn border_color(&mut self, v: BorderColor) -> &mut dyn SamplerBuilder;

    /// Set whether texture coordinates are normalized to the range `[0.0, 1.0]`.
    ///
    /// When set to `true`, the following conditions must met or the results of
    /// sampling are undefined:
    ///
    ///  - `min_filter` and `mag_filter` must be equal.
    ///  - `lod_clamp` must be `0.0 .. 0.0`.
    ///  - `max_anisotropy` must be `1`.
    ///  - Image views the sampler is used to sample must be 1D or 2D image
    ///    views and must have only a single layer and a single mipmap level.
    ///  - When sampling an image using the sampler, projection and constant
    ///    offsets cannot be used.
    fn unnorm_coords(&mut self, v: bool) -> &mut dyn SamplerBuilder;

    /// Build an `SamplerRef`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<SamplerRef>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorderColor {
    FloatTransparentBlack,
    FloatOpaqueBlack,
    FloatOpaqueWhite,
    IntTransparentBlack,
    IntOpaqueBlack,
    IntOpaqueWhite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Filter {
    Nearest,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MipmapMode {
    Nearest,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorderColor,
    MirroredClampToEdge,
}
