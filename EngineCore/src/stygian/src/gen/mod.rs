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
mod voxelize;

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
    /// Get the dimensions (including depth) of a tile.
    pub fn tile_size(&self) -> Vector3<u32> {
        self.tile_size.extend(self.depth)
    }

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
    /// The element `[[x, y]]` describes the tile `(x, y)`.
    pub(crate) tiles: Array2<BinnedGeometryTile>,
    pub(crate) polygon_store: Box<dyn MemStore<Polygon>>,
}

#[derive(Debug, Default)]
pub(crate) struct BinnedGeometryTile {
    pub(crate) polygon_page_ids: Vec<MemPageId<Polygon>>,
}

pub(crate) type Polygon = [Point3<f32>; 3];

/// A RLE-encoded voxel bitmap.
#[derive(Debug)]
pub struct VoxelBitmap {
    /// The element `[[x, y]]` describes the tile `(x, y)`.
    pub(crate) tiles: Array2<VoxelBitmapTile>,

    /// The memory store where each page contains an RLE-encoded voxel bitmap
    /// for a tile.
    ///
    /// Each row is associated with a sequence of one or more `Span`s.
    /// The sequence is terminated by a `Span` whose Z value is equal to the
    /// depth of the domain.
    pub(crate) rle_store: Box<dyn MemStore<Span>>,

    /// The memory store where each page contains a mapping from tile-local
    /// coordinates to indices into `rle_store`'s corresponding page.
    ///
    /// An index range is calculated as
    /// `page[x + y * tile_size.x] .. page[x + y * tile_size.x + 1]`.
    pub(crate) rle_index_store: Box<dyn MemStore<usize>>,
}

#[derive(Debug, Default)]
pub(crate) struct VoxelBitmapTile {
    pub(crate) rle_page_id: MemPageId<Span>,
    pub(crate) rle_index_page_id: MemPageId<usize>,
}

/// A span of consecutive voxels having the same attribute.
///
/// - The first element represents the type of the voxels.
/// - THe second element is the exclusive upper bound of the Z coordinates.
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct Span(SpanType, u16);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum SpanType {
    Empty,
    Solid,
    View,
}