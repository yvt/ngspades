//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::vec3;
use ndarray::Array2;
use std::{mem::replace, ops::Range};

use super::{
    utils::{bitarray_clear_range, bitarray_enum_spans, bitarray_set_range, BITS},
    InitialDomain, Span, VoxelBitmap, VoxelType,
};
use crate::{
    mempool::MemPageRefExt,
    terrain::{Terrain, TerrainLevel},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainConversionError {
    UnsupportedSize,
}

impl VoxelBitmap {
    /// Create a `Terrain` from this `VoxelBitmap`.
    ///
    /// The domain is downsampled in the X/Y directions by `2 ** downsample`.
    /// `downsample` must be equal to or less than `initial_domain.tile_size_bits`.
    /// The final dimensions must be powers of two.
    pub fn to_terrain(
        &self,
        initial_domain: &InitialDomain,
        downsample: u32,
    ) -> Result<Terrain, TerrainConversionError> {
        assert!(downsample <= initial_domain.tile_size_bits);

        let size = initial_domain.size();
        let out_size = vec3(size.x >> downsample, size.y >> downsample, size.z);

        if !(size.x.is_power_of_two() && size.x.is_power_of_two() && size.x.is_power_of_two()) {
            return Err(TerrainConversionError::UnsupportedSize);
        }

        let mut buffer = IntersectBuffer::new(size.z);

        let base_tiles = self.tiles.map(|tile_info| {
            let tile_rle_page = self.rle_store.get_page(tile_info.rle_page_id);
            let tile_rle_index_page = self.rle_index_store.get_page(tile_info.rle_index_page_id);
            let tile_rle = tile_rle_page.read();
            let tile_rle_index = tile_rle_index_page.read();
            convert_tile(
                &tile_rle,
                &tile_rle_index,
                initial_domain.tile_size_bits,
                downsample,
                &mut buffer,
                size.z,
            )
        });

        let mut base_level = TerrainLevel {
            rows: vec![Vec::new(); (out_size.x * out_size.y) as usize],
        };

        let mut tile_size = initial_domain.tile_size().cast::<usize>().unwrap();
        tile_size.x >>= downsample;
        tile_size.y >>= downsample;
        for ((tile_x, tile_y), tile) in { base_tiles }.indexed_iter_mut() {
            for ((y, x), row) in tile.indexed_iter_mut() {
                let row = replace(row, Vec::new());
                base_level.rows[(tile_x * tile_size.x + x)
                    + (tile_y * tile_size.y + y) * out_size.x as usize] = row;
            }
        }

        Ok(Terrain::from_base_level(
            out_size.cast::<usize>().unwrap(),
            base_level,
        ))
    }
}

fn convert_tile(
    tile_rle: &[Span],
    tile_rle_index: &[usize],
    size_bits: u32,
    downsample: u32,
    buffer: &mut IntersectBuffer,
    depth: u32,
) -> Array2<Vec<Range<u16>>> {
    let size = 1 << size_bits;
    let out_size = 1 << (size_bits - downsample);

    Array2::from_shape_fn((out_size, out_size), |(y, x)| {
        buffer.clear(depth);
        for y in y << downsample..y + 1 << downsample {
            for x in x << downsample..x + 1 << downsample {
                let idx = &tile_rle_index[(x + size * y) as usize..][..2];
                let (idx_start, idx_end) = (idx[0], idx[1]);
                buffer.carve(&tile_rle[idx_start..idx_end]);
            }
        }
        let mut v = Vec::new();
        buffer.to_rle(&mut v);
        v
    })
}

struct IntersectBuffer {
    bits: Vec<usize>,
}

impl IntersectBuffer {
    fn new(depth: u32) -> Self {
        Self {
            bits: vec![0; ((depth + BITS - 1) / BITS) as usize],
        }
    }

    fn clear(&mut self, depth: u32) {
        bitarray_set_range(&mut self.bits, 0..depth);
    }

    fn carve(&mut self, row: &[Span]) {
        let mut span_z_start = 0;
        for &Span(voxel_type, span_z_end) in row.iter() {
            let span_z_end = span_z_end as u32;

            if voxel_type == VoxelType::Empty {
                bitarray_clear_range(&mut self.bits, span_z_start..span_z_end);
            }

            span_z_start = span_z_end;
        }
    }

    fn to_rle(&self, out_rle: &mut Vec<Range<u16>>) {
        let mut z_start = 0;
        bitarray_enum_spans(
            &self.bits,
            self.bits.len() as u32 * BITS,
            |z_end, is_solid| {
                let z_end = z_end as u16;
                if is_solid {
                    out_rle.push(z_start..z_end);
                }
                z_start = z_end;
            },
        );
    }
}
