//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides miscellaneous functions.
use std::{fmt, cmp::min};

use super::{Span, VoxelBitmap, VoxelType};

impl VoxelBitmap {
    pub(crate) fn prefetch_tile(&self, tile: [u32; 2]) {
        let tile = &self.tiles[[tile[0] as usize, tile[1] as usize]];
        self.rle_store.prefetch_page(&[tile.rle_page_id]);
        self.rle_index_store
            .prefetch_page(&[tile.rle_index_page_id]);
    }
}

impl fmt::Debug for VoxelBitmap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::mempool::MemPageRefExt;

        if !f.alternate() {
            return f
                .debug_struct("VoxelBitmap")
                .field("tiles", &self.tiles)
                .field("rle_store", &self.rle_store)
                .field("rle_index_store", &self.rle_index_store)
                .finish();
        }

        writeln!(f, "VoxelBitmap {{\"")?;
        for (tile_id, tile_info) in self.tiles.indexed_iter() {
            writeln!(f, "[Tile {:?}]", tile_id)?;

            let tile_rle_page = self.rle_store.get_page(tile_info.rle_page_id);
            let tile_rle_index_page = self.rle_index_store.get_page(tile_info.rle_index_page_id);
            let tile_rle = tile_rle_page.read();
            let tile_rle_index = tile_rle_index_page.read();
            let size = (tile_rle_index.len() as f64).sqrt() as usize;

            for y in 0..size {
                writeln!(
                    f,
                    "  [Slice [{:?}, {:?}, ..]]",
                    size * tile_id.0..size * (tile_id.0 + 1),
                    y + size * tile_id.1,
                )?;
                for x in 0..size {
                    write!(f, "    ")?;

                    let idx = &tile_rle_index[(x + size * y) as usize..][..2];
                    let (idx_start, idx_end) = (idx[0], idx[1]);
                    let spans = &tile_rle[idx_start..idx_end];
                    let mut z = 0;

                    for &Span(voxel_type, span_z_end) in spans.iter() {
                        while z < span_z_end {
                            let c = match voxel_type {
                                VoxelType::Empty => ' ',
                                VoxelType::Solid => '#',
                                VoxelType::View => 'V',
                            };
                            write!(f, "{}", c)?;
                            z += 1;
                        }
                    }

                    writeln!(f)?;
                }
            }
        }
        write!(f, "\"}}")?;

        Ok(())
    }
}

/// The number of bits in `usize` (an element of `BitArray::buffer`).
pub const BITS: u32 = std::mem::size_of::<usize>() as u32 * 8;

pub fn bitarray_set_range(b: &mut [usize], range: std::ops::Range<u32>) {
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
            b[(start / BITS) as usize] |= !ones(start);
            start = next_boundary;
            next_boundary += BITS;
        }
    }
}

pub fn bitarray_clear_range(b: &mut [usize], range: std::ops::Range<u32>) {
    let (mut start, end) = (range.start, range.end);
    let mut next_boundary = (start / BITS + 1) * BITS;

    let end = min(end, b.len() as u32 * BITS);
    if start >= end {
        return;
    }

    loop {
        if end <= next_boundary {
            b[(start / BITS) as usize] &= !(ones(start) ^ ones2(end));
            break;
        } else {
            b[(start / BITS) as usize] &= ones(start);
            start = next_boundary;
            next_boundary += BITS;
        }
    }
}

pub fn bitarray_enum_spans(array: &[usize], end: u32, mut cb: impl FnMut(u32, bool)) {
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
pub fn one(x: u32) -> usize {
    1usize.wrapping_shl(x)
}

/// [0, x % BITS]
///
/// |  x | out                                 |
/// | -- | ----------------------------------- |
/// |  0 | 00000000 00000000 00000000 00000000 |
/// |  8 | 00000000 00000000 00000000 11111111 |
/// | 31 | 11111111 11111111 11111111 11111111 |
/// | 32 | 00000000 00000000 00000000 00000000 |
#[inline]
pub fn ones(x: u32) -> usize {
    one(x) - 1
}

/// [0, (x - 1) % BITS + 1]
///
/// |  x | out                                 |
/// | -- | ----------------------------------- |
/// |  1 | 00000000 00000000 00000000 00000001 |
/// | 31 | 01111111 11111111 11111111 11111111 |
/// | 32 | 11111111 11111111 11111111 11111111 |
#[inline]
pub fn ones2(x: u32) -> usize {
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

        let mut bits = [0usize; 2];
        bitarray_set_range(&mut bits, 4..BITS + 4);
        assert_eq!(!(bits[0] | 0b1111), 0, "{:?}", &bits);
        assert_eq!(bits[1], 0b1111, "{:?}", &bits);
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
