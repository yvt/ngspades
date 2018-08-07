//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::borrow::Borrow;
use {utils, ColoredVoxels, RowSolidVoxels};
use super::Row;

/// An iterator over the chunks in a `Row`.
///
/// Note that this does not implement `Iterator`.
#[derive(Debug, Clone)]
pub struct RowChunkIter<T>(T, usize, IterState, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IterState {
    Colored,
    UncoloredOrEnd,
}

/// An iterator over the consecutive colored/uncolored solid voxels in a chunk.
///
/// Each element indicates the starting Z coordinate and `RowSolidVoxels<T>`.
#[derive(Debug)]
pub struct RowChunkSolidVoxelsIter<'a, T: 'a>(&'a mut RowChunkIter<T>);

impl<'a, T: Borrow<[u8]>> Row<&'a T> {
    /// Get an iterator over the chunks in the row.
    pub fn chunks(&self) -> RowChunkIter<&'a [u8]> {
        let inner = self.1.borrow();
        RowChunkIter(inner, 0, IterState::Colored, 0)
    }
}

impl<'a, T: Borrow<[u8]>> Row<&'a mut T> {
    /// Get an iterator over the chunks in the row.
    pub fn chunks(&self) -> RowChunkIter<&[u8]> {
        let inner = (self.1 as &T).borrow();
        RowChunkIter(inner, 0, IterState::Colored, 0)
    }
}

impl<T> RowChunkIter<T> {
    pub fn offset(&self) -> usize {
        self.1
    }

    pub fn get_inner_ref(&self) -> &T {
        &self.0
    }

    pub fn get_inner_ref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<'a> RowChunkIter<&'a [u8]> {
    pub fn next(&mut self) -> Option<RowChunkSolidVoxelsIter<&'a [u8]>> {
        // Skip the terminator
        if self.1 > 0 && self.1 < self.0.len() {
            for _ in RowChunkSolidVoxelsIter(self) {}
            self.1 += 2;
            self.2 = IterState::Colored;
        }

        // do we have more chunks?
        if self.1 < self.0.len() {
            // Skip empty voxels
            let empty_count = unsafe { utils::load_u16_le(self.0, self.1) } as usize;
            self.3 += empty_count;
            self.1 += 2;

            Some(RowChunkSolidVoxelsIter(self))
        } else {
            None
        }
    }
}

impl<'a, 'b: 'a> Iterator for RowChunkSolidVoxelsIter<'a, &'b [u8]> {
    type Item = (usize, RowSolidVoxels<&'b [u8]>);

    fn next(&mut self) -> Option<Self::Item> {
        let ref mut iter = self.0;
        let slice = iter.0.as_ref();
        let count = unsafe { utils::load_u16_le(slice, iter.1) } as usize;
        if count == 0 {
            // End of chunk
            None
        } else {
            let z = iter.3;
            iter.3 += count;
            match iter.2 {
                IterState::Colored => {
                    let colored_slice = &slice[iter.1 + 2..][..count * 4];
                    iter.1 += 2 + count * 4;
                    iter.2 = IterState::UncoloredOrEnd;
                    Some((
                        z,
                        RowSolidVoxels::Colored(ColoredVoxels::new(colored_slice)),
                    ))
                }
                IterState::UncoloredOrEnd => {
                    iter.1 += 2;
                    iter.2 = IterState::Colored;
                    Some((z, RowSolidVoxels::Uncolored(count)))
                }
            }
        }
    }
}
