//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! # Stygian
//!
//! Occlusion culling using conservative voxel rasterization
//!
//! !["Me and My Shadow"](https://derpicdn.net/img/2018/8/24/1815256/large.png)

mod debug;
mod mipbeamcast;
mod terrain;
mod terrainrast;

pub use crate::{
    debug::{NoTrace, Trace},
    terrain::Terrain,
    terrainrast::TerrainRast,
};

mod utils {
    pub mod float;
    pub mod geom;
    pub mod int;
}

pub mod io {
    pub mod ngsterrain;
}
