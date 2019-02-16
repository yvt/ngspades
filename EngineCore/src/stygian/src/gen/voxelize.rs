//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use alt_fp::FloatOrdSet;
use cgmath::{vec2, Vector3};
use ndarray::{s, Array3};
use std::cmp::min;

use super::{
    tri::tricrast, BinnedGeometry, InitialDomain, Polygon, Span, VoxelType, VoxelBitmap,
    VoxelBitmapTile,
};
use crate::mempool::{MemPageRefExt, MemPool};

impl VoxelBitmap {
    /// Construct a `VoxelBitmap` by voxelizing a given [`BinnedGeometry`].
    pub fn from_geometry(
        pool: &impl MemPool,
        initial_domain: &InitialDomain,
        geometry: &BinnedGeometry,
    ) -> Self {
        let mut voxelizer = Voxelizer::new(initial_domain.tile_size());

        let rle_store = pool.new_store();
        let rle_index_store = pool.new_store();

        rle_store.set_name("RLE voxels");
        rle_index_store.set_name("RLE voxel index");

        let mut rle = Vec::new();
        let mut rle_index = Vec::new();

        let tiles = geometry.tiles.map(|tile| {
            voxelizer.clear();
            for &page_id in tile.polygon_page_ids.iter() {
                let polygons = geometry.polygon_store.get_page(page_id).read();
                for &p in polygons.iter() {
                    voxelizer.draw_polygon(p);
                }
            }

            voxelizer.to_rle(&mut rle, &mut rle_index);

            let rle_page_id = rle_store.new_page(rle.len());
            let rle_index_page_id = rle_index_store.new_page(rle_index.len());

            (rle_store.get_page(rle_page_id).write())
                .as_vec()
                .extend(rle.drain(..));
            (rle_index_store.get_page(rle_index_page_id).write())
                .as_vec()
                .extend(rle_index.drain(..));

            VoxelBitmapTile {
                rle_page_id,
                rle_index_page_id,
            }
        });

        Self {
            rle_store,
            rle_index_store,
            tiles,
        }
    }
}

#[derive(Debug)]
struct Voxelizer {
    /// `[y, x, z / BITS]`
    bitmap: Array3<usize>,
    /// Temporary storage for `tricrast`.
    z_buffer: Vec<std::ops::Range<f32>>,
    z_max_f: f32,
    depth: u32,
}

impl Voxelizer {
    fn new(size: Vector3<u32>) -> Self {
        assert!(size.z < 65536, "{} < 65536", size.z);
        Self {
            bitmap: Array3::zeros([
                size.y as usize,
                size.x as usize,
                ((size.z + BITS - 1) / BITS) as usize,
            ]),
            z_buffer: vec![0.0..0.0; size.x as usize],
            z_max_f: size.z as f32,
            depth: size.z,
        }
    }

    fn clear(&mut self) {
        for x in self.bitmap.iter_mut() {
            *x = 0;
        }
    }

    fn draw_polygon(&mut self, p: Polygon) {
        let bitmap = &mut self.bitmap;
        let z_max_f = self.z_max_f;

        // Voxelize the given triangle
        tricrast(
            p,
            vec2(bitmap.shape()[1] as u32, bitmap.shape()[0] as u32),
            &mut self.z_buffer,
            |origin, z_ranges| {
                let y = origin.y as usize;
                for (x, z_range) in (origin.x as usize..).zip(z_ranges.iter()) {
                    let z_min = [z_range.start, 0.0].fmax() as i32;
                    let z_max = [z_range.end.ceil(), z_max_f].fmin() as i32;
                    if z_min >= z_max {
                        continue;
                    }

                    let (z_min, z_max) = (z_min as u32, z_max as u32);

                    let mut row = bitmap.slice_mut(s![y, x, ..]);
                    let row_slice = row.as_slice_mut().unwrap();
                    bitarray_set_range(row_slice, z_min..z_max);
                }
            },
        );
    }

    fn to_rle(&self, out_rle: &mut Vec<Span>, out_rle_index: &mut Vec<usize>) {
        let shape = self.bitmap.shape();
        for y in 0..shape[0] {
            for x in 0..shape[1] {
                let row = self.bitmap.slice(s![y, x, ..]);
                let row_slice = row.as_slice().unwrap();
                out_rle_index.push(out_rle.len());
                bitarray_enum_spans(row_slice, self.depth, |z_end, is_solid| {
                    let span_type = if is_solid {
                        VoxelType::Solid
                    } else {
                        VoxelType::Empty
                    };
                    out_rle.push(Span(span_type, z_end as u16));
                });
            }
        }
        out_rle_index.push(out_rle.len());
    }
}

/// The number of bits in `usize` (an element of `BitArray::buffer`).
const BITS: u32 = std::mem::size_of::<usize>() as u32 * 8;

fn bitarray_set_range(b: &mut [usize], range: std::ops::Range<u32>) {
    let (mut start, end) = (range.start, range.end);
    let mut next_boundary = (start / BITS + 1) * BITS;

    let end = min(end, b.len() as u32 * BITS);
    if start >= end {
        return;
    }

    loop {
        if end <= next_boundary {
            b[(start / BITS) as usize] |= ones(start) ^ ones2(end);
            break;
        } else {
            b[(start / BITS) as usize] |= ones(start);
            start = next_boundary;
            next_boundary += BITS;
        }
    }
}

fn bitarray_enum_spans(array: &[usize], end: u32, mut cb: impl FnMut(u32, bool)) {
    let mut start = 0;
    let mut next_boundary = BITS;

    let mut mask = 0usize.wrapping_sub(array[0] & 0);
    let mut bits = array[0];

    loop {
        let count = (bits ^ mask).trailing_zeros();

        if start + count >= next_boundary {
            if next_boundary >= end {
                cb(end, mask != 0);
                return;
            }
            start = next_boundary;
            next_boundary += BITS;
            bits = array[(start / BITS) as usize];
            continue;
        }

        start += count;
        if start >= end {
            cb(end, mask != 0);
            return;
        }

        cb(start, mask != 0);
        mask = !mask;
        bits >>= count;
    }
}

#[inline]
fn one(x: u32) -> usize {
    1usize.wrapping_shl(x)
}

// [0, x % BITS]
//
// |  x | out                                 |
// | -- | ----------------------------------- |
// |  0 | 00000000 00000000 00000000 00000000 |
// |  8 | 00000000 00000000 00000000 11111111 |
// | 31 | 11111111 11111111 11111111 11111111 |
// | 32 | 00000000 00000000 00000000 00000000 |
#[inline]
fn ones(x: u32) -> usize {
    one(x) - 1
}

// [0, (x - 1) % BITS + 1]
//
// |  x | out                                 |
// | -- | ----------------------------------- |
// |  1 | 00000000 00000000 00000000 00000001 |
// | 31 | 01111111 11111111 11111111 11111111 |
// | 32 | 11111111 11111111 11111111 11111111 |
#[inline]
fn ones2(x: u32) -> usize {
    2usize.wrapping_shl(x.wrapping_sub(1)).wrapping_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitarray_set_range_sanity() {
        let mut bits = [0usize];
        bitarray_set_range(&mut bits, 4..8);
        assert_eq!(bits[0], 0b11110000);
    }

    #[test]
    fn bitarray_enum_spans_sanity() {
        let mut bits = [0usize; 4];
        bitarray_set_range(&mut bits, 4..15);
        bitarray_set_range(&mut bits, 20..36);
        bitarray_set_range(&mut bits, 50..52);

        dbg!(&bits);

        let mut results = Vec::new();
        bitarray_enum_spans(&bits, 54, |z_end, set| {
            results.push((z_end, set));
        });
        assert_eq!(
            &results[..],
            &[
                (4, false),
                (15, true),
                (20, false),
                (36, true),
                (50, false),
                (52, true),
                (54, false),
            ][..]
        );
    }
}
