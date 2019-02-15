//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides an implementation of the terrain generator.
use cgmath::{vec3, Point3, Vector2, Vector3};
use ndarray::Array2;

use crate::mempool::{MemPageId, MemStore};

mod binner;
mod tri;

pub use self::binner::PolygonBinner;

/// Defines an initial domain.
///
/// An initial domain describes the organization of the data used in the
/// first part (before downsampling) of the generation process. It is comprised
/// of `tile_count.x * tile_count.y` tiles arranged on a X-Y grid. Each tile is
/// an AABB whose dimensions are `tile_size.extend(depth)` and associated
/// with a voxel bitmap of size `tile_size.extend(depth)`.
#[derive(Debug, Copy, Clone)]
pub struct InitialDomain {
    pub tile_size: Vector2<u32>,
    pub depth: u32,
    pub tile_count: Vector2<u32>,
}

impl InitialDomain {
    /// Get the size of an initial domain.
    pub fn size(&self) -> Vector3<u32> {
        let InitialDomain {
            tile_size,
            depth,
            tile_count,
        } = self;
        vec3(
            tile_size.x * tile_count.x,
            tile_size.y * tile_count.y,
            *depth,
        )
    }
}

/// A binned polygon soup (an unorganized set of polygons).
#[derive(Debug)]
pub struct BinnedGeometry {
    pub(crate) tiles: Array2<BinnedGeometryTile>,
    pub(crate) polygon_store: Box<dyn MemStore<Polygon>>,
}

#[derive(Debug, Default)]
pub(crate) struct BinnedGeometryTile {
    pub(crate) polygon_page_ids: Vec<MemPageId<Polygon>>,
}

pub(crate) type Polygon = [Point3<f32>; 3];
