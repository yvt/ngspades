//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{vec3, Vector3};
use std::cmp::min;

/// A voxelmap describing occluders.
#[derive(Debug, Clone)]
pub struct Terrain {
    /// Indicates the size of the voxelmap. The actual dimension for each axis
    /// is `1 << x` where `x` is the value of the corresponding element.
    ///
    /// No element can be greater-or-equal than `16`.
    pub(crate) size_bits: Vector3<u32>,

    /// Mip levels of a terrain. The number of elements (mip levels) is
    /// equal to `1 + min(size_bits.x, size_bits.y)`.
    pub(crate) levels: Vec<TerrainLevel>,
}

/// A single mip level of `Terrain`.
#[derive(Debug, Clone)]
pub(crate) struct TerrainLevel {
    /// The RLE representation of rows. A row is a set of voxels at
    /// particular X and Y coordinates.
    ///
    /// The number of valid elements of the top-level `Vec` is equal to
    /// `(1 << size_bits.x) * (1 << size_bits.y)` (if `level == 0`) or
    /// `((1 << size_bits.x + 1 - level) - 1) * ((1 << size_bits.y + 1 - level) - 1)` (otherwise).
    /// The pitch (the distance between rows) is `size().x >> max(0, level - 1)`,
    /// which is off by one when `level > 0`, but is faster to compute.
    /// Elements are indexed by a row's X and Y coordinates using the
    /// formula: `x + y * (size().x >> max(0, level - 1))`.
    ///
    /// Each element of the top-level `Vec` is a `Vec` containing zero or more
    /// `Span`s in a row. Spans must be sorted by their Z coordinates in an
    /// ascending order.
    pub(crate) rows: Vec<Vec<Span>>,
}

/// A set of one or more consecutive solid voxels on a line parallel to the Z axis.
pub(crate) type Span = std::ops::Range<u16>;

impl Terrain {
    pub fn size(&self) -> Vector3<usize> {
        vec3(
            1 << self.size_bits.x,
            1 << self.size_bits.y,
            1 << self.size_bits.z,
        )
    }
}

// Terrain bulk loader - each mip level is generated based on the base level
impl Terrain {
    pub(crate) fn from_base_level(size: Vector3<usize>, base_level: TerrainLevel) -> Self {
        assert!(size.x.is_power_of_two());
        assert!(size.y.is_power_of_two());
        assert!(size.z.is_power_of_two());

        // If the terrain was too large, inputs to `alt_fp::u23_to_f32` would
        // overflow
        assert!(size.x < (1 << 20));
        assert!(size.y < (1 << 20));
        assert!(size.z < (1 << 20));

        let size_bits = vec3(
            size.x.trailing_zeros(),
            size.y.trailing_zeros(),
            size.z.trailing_zeros(),
        );

        let num_levels = min(size_bits.x, size_bits.y) + 1;
        let mut levels = Vec::with_capacity(num_levels as usize);

        levels.push(base_level);

        // Generate level 1
        let mut row_downsampler = RowDownsampler::new(size.z);
        let mut last_size = size;
        if num_levels >= 2 {
            let (next_level, next_size) = levels
                .last()
                .unwrap()
                .level1(last_size, &mut row_downsampler);

            levels.push(next_level);
            last_size = next_size;
        }

        // Generate the rest of mip levels
        for _ in 2..num_levels {
            let (next_level, next_size) = levels
                .last()
                .unwrap()
                .downsample(last_size, &mut row_downsampler);

            levels.push(next_level);
            last_size = next_size;
        }

        Self { size_bits, levels }
    }
}

impl TerrainLevel {
    /// Generate a level 1 mipmap.
    fn level1(
        &self,
        size: Vector3<usize>,
        row_downsampler: &mut RowDownsampler,
    ) -> (Self, Vector3<usize>) {
        debug_assert!(size.x >= 2 && size.y >= 2, "{:?}", size);
        debug_assert!(size.x % 2 == 0, "{:?}", size);
        debug_assert!(size.y % 2 == 0, "{:?}", size);

        let out_size = vec3(size.x - 1, size.y - 1, size.z);

        let mut out_rows = Vec::with_capacity((out_size.x + 1) * out_size.y);
        for out_y in 0..out_size.y {
            for out_x in 0..out_size.x {
                let in_x1 = out_x;
                let in_y1 = out_y;
                let in_x2 = in_x1 + 1;
                let in_y2 = in_y1 + 1;

                let in_rows = [
                    &self.rows[in_x1 + in_y1 * size.x],
                    &self.rows[in_x2 + in_y1 * size.x],
                    &self.rows[in_x1 + in_y2 * size.x],
                    &self.rows[in_x2 + in_y2 * size.x],
                ];

                out_rows.push(row_downsampler.downsample(&in_rows));
            }
            out_rows.push(Vec::new()); // pitch is off by one
        }

        (Self { rows: out_rows }, out_size)
    }

    /// Generate a level 2 or greater mipmap.
    fn downsample(
        &self,
        size: Vector3<usize>,
        row_downsampler: &mut RowDownsampler,
    ) -> (Self, Vector3<usize>) {
        debug_assert!(size.x >= 3 && size.y >= 3, "{:?}", size);
        debug_assert!(size.x % 2 == 1, "{:?}", size);
        debug_assert!(size.y % 2 == 1, "{:?}", size);

        let out_size = vec3(size.x / 2, size.y / 2, size.z);
        debug_assert!(out_size.x % 2 == 1, "{:?}", out_size);
        debug_assert!(out_size.y % 2 == 1, "{:?}", out_size);

        let mut out_rows = Vec::with_capacity((out_size.x + 1) * out_size.y);
        for out_y in 0..out_size.y {
            for out_x in 0..out_size.x {
                let in_x1 = out_x * 2;
                let in_y1 = out_y * 2;
                let in_x2 = in_x1 + 2;
                let in_y2 = in_y1 + 2;

                let in_rows = [
                    &self.rows[in_x1 + in_y1 * (size.x + 1)],
                    &self.rows[in_x2 + in_y1 * (size.x + 1)],
                    &self.rows[in_x1 + in_y2 * (size.x + 1)],
                    &self.rows[in_x2 + in_y2 * (size.x + 1)],
                ];

                out_rows.push(row_downsampler.downsample(&in_rows));
            }
            out_rows.push(Vec::new()); // pitch is off by one
        }

        (Self { rows: out_rows }, out_size)
    }
}

struct RowDownsampler {
    voxels: Vec<u8>,
}

impl RowDownsampler {
    fn new(size: usize) -> Self {
        Self {
            voxels: vec![0; size],
        }
    }

    fn downsample(&mut self, rows: &[&Vec<Span>; 4]) -> Vec<Span> {
        let voxels = &mut self.voxels;

        for count in voxels.iter_mut() {
            *count = 0;
        }

        // FIXME: This could be optimized by tracking endpoints rather than
        // rasterizing the rows
        for row in rows.iter() {
            for range in row.iter() {
                for z in range.clone() {
                    voxels[z as usize] += 1;
                }
            }
        }

        // Get the intersection of given rows
        let mut out = Vec::new();
        let mut z = 0;
        let num_in_rows = rows.len() as u8;
        while z < voxels.len() {
            if voxels[z] == num_in_rows {
                // Occupied by all rows - start a span
                let start = z as _;

                z += 1;
                while z < voxels.len() && voxels[z] == num_in_rows {
                    z += 1;
                }

                out.push(start..z as _);
            } else {
                z += 1;
            }
        }

        out
    }
}
