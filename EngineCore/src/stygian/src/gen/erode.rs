//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ndarray::Array2;
use std::num::Wrapping;

use super::{
    utils::{bitarray_clear_range, bitarray_enum_spans, bitarray_set_range, BITS},
    InitialDomain, Span, VoxelBitmap, VoxelBitmapTile, VoxelType,
};
use crate::mempool::{MemPageRef, MemPageRefExt, MemPool, ReadGuard};

const PLUS_ONE: u32 = 1;
const MINUS_ONE: u32 = 0xffffffff;
const DIRS: [[u32; 2]; 9] = [
    [MINUS_ONE, MINUS_ONE],
    [0, MINUS_ONE],
    [PLUS_ONE, MINUS_ONE],
    [MINUS_ONE, 0],
    [0, 0],
    [PLUS_ONE, 0],
    [MINUS_ONE, PLUS_ONE],
    [0, PLUS_ONE],
    [PLUS_ONE, PLUS_ONE],
];

impl VoxelBitmap {
    /// Erode the voxels of type `Empty` and `Solid` (complementary to `View`)
    /// and create a new `VoxelBitmap` containing the result.
    ///
    /// ```text
    /// (V = View, # = Solid, . = Empty)
    /// Input:         V V V V # . . . . # V # # # V
    /// Empty & Solid: . . . . # # # # # # . # # # .
    /// Output:        . . . . . # # # # . . . # . .
    /// ```
    ///
    pub fn erode_view(&self, pool: &impl MemPool, initial_domain: &InitialDomain) -> VoxelBitmap {
        let rle_store = pool.new_store();
        let rle_index_store = pool.new_store();

        rle_store.set_name("RLE voxels");
        rle_index_store.set_name("RLE voxel index");

        let tile_count = initial_domain.tile_count;
        let tile_size = initial_domain.tile_size();
        let depth = initial_domain.depth;
        let mut tiles: Array2<VoxelBitmapTile> =
            Array2::default([tile_count.x as usize, tile_count.y as usize]);

        let mut buffer = ErodeBuffer::new(depth);
        let mut rle = Vec::new();
        let mut rle_index = Vec::new();

        for (tile_id, _) in self.tiles.indexed_iter() {
            // 8-connected neighbors
            let tile_x = Wrapping(tile_id.0 as u32);
            let tile_y = Wrapping(tile_id.1 as u32);
            let one = Wrapping(1);
            let tile_refs = [
                [
                    self.get_tile_ref(tile_x - one, tile_y - one),
                    self.get_tile_ref(tile_x, tile_y - one),
                    self.get_tile_ref(tile_x + one, tile_y - one),
                    None,
                ],
                [
                    self.get_tile_ref(tile_x - one, tile_y),
                    self.get_tile_ref(tile_x, tile_y),
                    self.get_tile_ref(tile_x + one, tile_y),
                    None,
                ],
                [
                    self.get_tile_ref(tile_x - one, tile_y + one),
                    self.get_tile_ref(tile_x, tile_y + one),
                    self.get_tile_ref(tile_x + one, tile_y + one),
                    None,
                ],
            ];

            for y in 0..tile_size.y {
                for x in 0..tile_size.x {
                    rle_index.push(rle.len());

                    buffer.clear(depth);

                    // Find the intersection of the current row and its
                    // eight connected rows. (Erosion in the X/Y direcitons)
                    for &[dx, dy] in DIRS.iter() {
                        let row_x = x.wrapping_add(dx);
                        let row_y = y.wrapping_add(dy);
                        // Relative tile ID - `1` = current column/row
                        let tile_rel_x = row_x.wrapping_add(tile_size.x) / tile_size.x;
                        let tile_rel_y = row_y.wrapping_add(tile_size.y) / tile_size.y;
                        let row_x = row_x % tile_size.x;
                        let row_y = row_y % tile_size.y;
                        let tile_ref = &tile_refs[tile_rel_y as usize][tile_rel_x as usize];
                        if let Some(tile_ref) = tile_ref {
                            let row = tile_ref.get_row(row_x, row_y, tile_size.x);
                            buffer.carve(row);
                        }
                    }

                    // Erode in the Z direction
                    buffer.erode();

                    buffer.to_rle(&mut rle);
                }
            }
            rle_index.push(rle.len());

            let rle_page_id = rle_store.new_page(rle.len());
            let rle_index_page_id = rle_index_store.new_page(rle_index.len());

            (rle_store.get_page(rle_page_id).write())
                .as_vec()
                .extend(rle.drain(..));
            (rle_index_store.get_page(rle_index_page_id).write())
                .as_vec()
                .extend(rle_index.drain(..));

            let out_tile_info = &mut tiles[tile_id];
            out_tile_info.rle_page_id = rle_page_id;
            out_tile_info.rle_index_page_id = rle_index_page_id;
        }

        Self {
            rle_store,
            rle_index_store,
            tiles,
        }
    }

    fn get_tile_ref(&self, x: Wrapping<u32>, y: Wrapping<u32>) -> Option<TileRef> {
        let (Wrapping(x), Wrapping(y)) = (x, y);
        self.tiles
            .get([x as usize, y as usize])
            .map(|tile_info| TileRef {
                rle: self.rle_store.get_page(tile_info.rle_page_id).read(),
                rle_index: self
                    .rle_index_store
                    .get_page(tile_info.rle_index_page_id)
                    .read(),
            })
    }
}

struct TileRef<'a> {
    rle: ReadGuard<'a, dyn MemPageRef<Span> + 'a, Span>,
    rle_index: ReadGuard<'a, dyn MemPageRef<usize> + 'a, usize>,
}

impl TileRef<'_> {
    fn get_row(&self, x: u32, y: u32, size: u32) -> &[Span] {
        let idx = &self.rle_index[(x + size * y) as usize..][..2];
        let (idx_start, idx_end) = (idx[0], idx[1]);
        &self.rle[idx_start..idx_end]
    }
}

struct ErodeBuffer {
    bits: Vec<usize>,
}

impl ErodeBuffer {
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

            if voxel_type == VoxelType::View {
                bitarray_clear_range(&mut self.bits, span_z_start..span_z_end);
            }

            span_z_start = span_z_end;
        }
    }

    fn erode(&mut self) {
        let bits = &mut self.bits[..];
        let mut last = 0;
        for i in 0..bits.len() {
            let old = bits[i];
            let next = bits.get(i + 1).cloned().unwrap_or(0);
            bits[i] = old & ((old >> 1) | (next << BITS - 1)) & ((old << 1) | last);
            last = old >> BITS - 1;
        }
    }

    fn to_rle(&self, out_rle: &mut Vec<Span>) {
        bitarray_enum_spans(
            &self.bits,
            self.bits.len() as u32 * BITS,
            |z_end, is_solid| {
                let span_type = if is_solid {
                    VoxelType::Solid
                } else {
                    VoxelType::Empty
                };
                out_rle.push(Span(span_type, z_end as u16));
            },
        );
    }
}
