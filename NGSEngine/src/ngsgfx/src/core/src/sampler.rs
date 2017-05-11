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

use super::CompareFunction;

pub trait Sampler: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {}

#[derive(Debug, Clone, Copy)]
pub struct SamplerDescription {
    pub mag_filter: Filter,
    pub min_filter: Filter,
    pub mipmap_mode: MipmapMode,
    pub address_mode: [SamplerAddressMode; 3],
    // TODO: mip lod bias? can't find in Metal...
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub max_anisotropy: i32,
    pub compare_function: Option<CompareFunction>,
    pub border_color: SamplerBorderColor,
    pub unnormalized_coordinates: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SamplerBorderColor {
    FloatTransparentBlock,
    FloatOpaqueBlack,
    FloatOpaqueWhite,
    IntTransparentBlock,
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


