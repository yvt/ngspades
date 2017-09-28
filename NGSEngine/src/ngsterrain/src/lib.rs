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

use cgmath::{Vector3, BaseNum};
use std::ops::Neg;

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

impl CubeFace {
    pub fn as_vector3<S: BaseNum + Neg<Output = S>>(&self) -> Vector3<S> {
        match self {
            &CubeFace::PositiveX => Vector3::unit_x(),
            &CubeFace::NegativeX => -Vector3::unit_x(),
            &CubeFace::PositiveY => Vector3::unit_y(),
            &CubeFace::NegativeY => -Vector3::unit_y(),
            &CubeFace::PositiveZ => Vector3::unit_z(),
            &CubeFace::NegativeZ => -Vector3::unit_z(),
        }
    }
}
