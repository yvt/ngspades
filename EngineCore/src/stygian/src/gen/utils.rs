//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides miscellaneous functions.
use std::fmt;

use super::{VoxelBitmap, VoxelType, Span};

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
