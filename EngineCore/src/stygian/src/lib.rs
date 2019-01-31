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
mod terrain;
mod mipbeamcast;

pub use crate::{terrain::Terrain, debug::{Trace, NoTrace}};

mod utils {
    pub mod float;
    pub mod int;
}

pub mod io {
    pub mod ngsterrain;
}
