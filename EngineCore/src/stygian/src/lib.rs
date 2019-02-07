//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! # Stygian
//!
//! Occlusion culling using conservative voxel rasterization
//!
//! !["Me and My Shadow"](https://derpicdn.net/img/2018/8/24/1815256/large.png)
//!
//! # Depth buffer range
//!
//! This library assumes that depth values are reversed — the near and far
//! regions are mapped to 1 and 0 respectively. This usage of a depth buffer is
//! called *the reversed floating-point buffer* and makes an optimal use of the
//! region of floating point values near 0 where representable numbers are
//! densely arranged.
//! If you use a traditional setup, you might have to manually modify a matrix
//! to reverse Z values before passing it to a library function.
mod debug;
mod depthimage;
mod mipbeamcast;
mod opticast;
mod terrain;
mod terrainrast;

pub use crate::{
    debug::{NoTrace, Trace},
    depthimage::DepthImage,
    terrain::Terrain,
    terrainrast::TerrainRast,
};

mod utils {
    pub mod float;
    pub mod geom;
}

pub mod io {
    pub mod ngsterrain;
}

/// The depth value of the far plane.
const DEPTH_FAR: f32 = 0.0;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../common/terrainload.rs"]
mod terrainload;
