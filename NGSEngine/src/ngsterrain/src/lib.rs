//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsTerrain / NgsTF
//! ==================
//!
//! Crate used to inspect and manipulate terrain data in the NgsTF (Nightingales
//! Terrain Format).
#![feature(platform_intrinsics)]
#![cfg_attr(test, feature(test))]

#[macro_use]
extern crate arrayref;
extern crate byteorder;
pub extern crate cgmath;

#[cfg(test)]
mod benchmark;

mod geom;
pub mod heightmap;
pub mod raytrace;
mod row;
mod terrain;
mod utils;
mod voxel;
mod voxels;

pub use self::row::*;
pub use self::terrain::*;
pub use self::voxel::*;
pub use self::voxels::*;

/// Specifies a face of cube.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CubeFace {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}