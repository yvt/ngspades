//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::borrow::Borrow;
use std::ops::Range;
use utils;
use super::Row;

/// An iterator over the chunks' Z value ranges in a `Row`.
///
/// Each element indicates a range of Z values.
#[derive(Debug)]
pub struct RowChunkRangeIter<'a>(&'a [u8], usize, usize);

impl<'a, T: Borrow<[u8]>> Row<&'a T> {
    /// Get an iterator over the chunks' Z value ranges in the row.
    pub fn chunk_z_ranges(&self) -> RowChunkRangeIter {
        let inner = self.1.borrow();
        RowChunkRangeIter(inner, 0, 0)
    }
}

impl<'a> Iterator for RowChunkRangeIter<'a> {
    type Item = Range<usize>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let s = self.0;
        if self.1 + 4 >= s.len() {
            None
        } else {
            // Skip empty voxels
            self.2 += unsafe { utils::load_u16_le(s, self.1) } as usize;
            self.1 += 2;

            // Read voxels
            let mut total_num_voxels = 0usize;

            // First colored voxels
            let num_voxels = unsafe { utils::load_u16_le(s, self.1) } as usize;
            debug_assert_ne!(num_voxels, 0);
            self.1 += 2 + num_voxels * 4;
            total_num_voxels += num_voxels;

            loop {
                // Uncolored
                let num_voxels = unsafe { utils::load_u16_le(s, self.1) } as usize;
                self.1 += 2;
                if num_voxels == 0 {
                    break;
                }
                total_num_voxels += num_voxels;

                // Colored
                let num_voxels = unsafe { utils::load_u16_le(s, self.1) } as usize;
                debug_assert_ne!(num_voxels, 0);
                self.1 += 2 + num_voxels * 4;
                total_num_voxels += num_voxels;
            }

            let range = self.2..self.2 + total_num_voxels;
            self.2 += total_num_voxels;

            Some(range)
        }
    }
}
