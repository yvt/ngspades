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

use {CompareFunction, Validate, DeviceCapabilities};

pub trait Sampler: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {}

#[derive(Debug, Clone, Copy)]
pub struct SamplerDescription {
    pub mag_filter: Filter,
    pub min_filter: Filter,
    pub mipmap_mode: MipmapMode,
    pub address_mode: [SamplerAddressMode; 3],
    // mip lod bias is intentionally excluded because it's a private API in Metal
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub max_anisotropy: u32,

    /// Specifies the comparison function used when sampling from a depth texture.
    ///
    /// `Some(Never)` will be treated as `None`.
    pub compare_function: Option<CompareFunction>,
    pub border_color: SamplerBorderColor,

    /// Specifies whether texture coordinates are normalized to the range `[0.0, 1.0]`.
    ///
    /// When set to `true`, the following conditions must met or the results of sampling are undefined:
    ///  - `min_filter` and `mag_filter` must be equal.
    ///  - `lod_min_clamp` and `lod_max_clamp` must be zero.
    ///  - `max_anisotropy` must be one.
    ///  - Image views the sampler is used to sample must be 1D or 2D image views and
    ///    must have only a single layer and a single mipmap level.
    ///  - When sampling an image using the sampler, projection and constant offsets cannot be used.
    pub unnormalized_coordinates: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SamplerBorderColor {
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
pub enum SamplerAddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorderColor,
    MirroredClampToEdge,
}

/// Validation errors for [`SamplerDescription`](struct.SamplerDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SamplerDescriptionValidationError {
    // TODO
}

impl Validate for SamplerDescription {
    type Error = SamplerDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
        where T: FnMut(Self::Error) -> ()
    {
        // TODO
    }
}


