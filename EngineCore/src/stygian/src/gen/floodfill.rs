//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Point3;
use iterpool::{Pool, PoolPtr};
use ndarray::Array2;
use std::{
    collections::{hash_map::Entry, HashMap, VecDeque},
    ops::Range,
    mem::replace,
};

use super::{InitialDomain, Span, VoxelBitmap, VoxelType};
use crate::mempool::MemPageRefExt;

const PLUS_ONE: u32 = 1;
const MINUS_ONE: u32 = 0xffffffff;
const DIRS: [[u32; 2]; 4] = [[PLUS_ONE, 0], [MINUS_ONE, 0], [0, PLUS_ONE], [0, MINUS_ONE]];

/// The internal state of the algorithm (possibly shared by threads).
#[derive(Debug)]
struct State {
    pending_tiles: HashMap<[u32; 2], TileState>,
    tile_queue: VecDeque<[u32; 2]>,
}

#[derive(Debug, Default)]
struct TileState {
    influx_spans: Vec<(u32, u32, Range<u32>)>,
}

impl VoxelBitmap {
    /// Perform a flood-fill operation on a `VoxelBitmap` in-place.
    pub fn flood_fill_in_place(
        &mut self,
        initial_domain: &InitialDomain,
        start_points: &[Point3<u32>],
        type_from: VoxelType,
        type_to: VoxelType,
    ) {
        let mut state = State {
            pending_tiles: HashMap::new(),
            tile_queue: VecDeque::new(),
        };
        let tile_size_bits = initial_domain.tile_size_bits;
        let tile_size = initial_domain.tile_size();

        // Insert the starting points into the queue
        for point in start_points.iter() {
            let tile_x = point.x / tile_size.x;
            let tile_y = point.y / tile_size.y;

            let entry = state.pending_tiles.entry([tile_x, tile_y]);
            if let Entry::Vacant(_) = entry {
                state.tile_queue.push_back([tile_x, tile_y]);
            }

            let tile_state = entry.or_default();
            tile_state.influx_spans.push((
                point.x % tile_size.x,
                point.y % tile_size.y,
                point.z..point.z + 1,
            ));
        }

        // Tile-local states
        #[derive(Debug, Clone)]
        struct LocalPendingSpan {
            z: Range<u32>,
            /// Forms a singly-linked list in `influx_spans`.
            next: Option<PoolPtr>,
        }

        // A queue of rows having an corresponding element in `influx_spans_head`
        // that is `Some(_)` (i.e., has at least one pending span).
        let mut row_queue = VecDeque::with_capacity((tile_size.x * tile_size.y) as usize);
        let mut influx_spans: Pool<LocalPendingSpan> =
            Pool::with_capacity(tile_size.x as usize * 4);
        let mut influx_spans_head: Array2<Option<PoolPtr>> =
            Array2::default([tile_size.x as usize, tile_size.y as usize]);

        // Spans to be pushed to adjacent tiles' `influx_spans`es
        let mut outflux_spans = Vec::new();;

        let mut range_flattener = RangeFlattener::with_capacity(16);
        let mut flattened_spans = Vec::new();

        if let Some(&tile_id) = state.tile_queue.front() {
            self.prefetch_tile(tile_id);
        }

        while state.tile_queue.len() > 0 {
            // Choose a tile to process
            let tile_id = state.tile_queue.pop_front().unwrap();
            let mut tile_state = state.pending_tiles.remove(&tile_id).unwrap();

            // Prefetch the next tile while we are working on the current one
            if let Some(&tile_id) = state.tile_queue.front() {
                self.prefetch_tile(tile_id);
            }

            // Load the tile
            let tile_info = &self.tiles[[tile_id[0] as usize, tile_id[1] as usize]];
            let tile_rle_page = self.rle_store.get_page(tile_info.rle_page_id);
            let tile_rle_index_page = self.rle_index_store.get_page(tile_info.rle_index_page_id);
            let mut tile_rle = tile_rle_page.write();
            let tile_rle_index = tile_rle_index_page.read();

            // Load influx spans from `TileState` and inject them into the queue
            for (x, y, z) in tile_state.influx_spans.drain(..) {
                let head = &mut influx_spans_head[[x as usize, y as usize]];
                if head.is_none() {
                    row_queue.push_back([x, y]);
                }

                let ptr = influx_spans.allocate(LocalPendingSpan { z, next: *head });
                *head = Some(ptr);
            }

            while let Some([row_x, row_y]) = row_queue.pop_front() {
                // Flatten influx spans of the current row. The output represents
                // an identical set of ranges but they are sorted by their
                // Z coordinates, enabling the use of a merge sort-like algorithm
                // in the following step.
                let mut ptr = replace(&mut influx_spans_head[[row_x as usize, row_y as usize]], None);
                while let Some(p) = ptr {
                    let LocalPendingSpan { z, next } = influx_spans.deallocate(p).unwrap();
                    range_flattener.insert(z);
                    ptr = next;
                }
                range_flattener.get_union(|range| flattened_spans.push(range));
                range_flattener.clear();

                // We'll iterate through the flattened influx spans.
                //
                // This'll panic if the set is empty, but that is not supposed
                // to be the case.
                let mut influx_span_it = flattened_spans.iter().cloned();
                let mut influx_span = influx_span_it.next().unwrap();

                // Fill empty spans adjacent to influx spans.
                let idx = &tile_rle_index[(row_x + tile_size.x * row_y) as usize..][..2];
                let (idx_start, idx_end) = (idx[0], idx[1]);
                let spans = &mut tile_rle[idx_start..idx_end];
                let mut span_z_start = 0;

                'fill_loop: for Span(voxel_type, span_z_end) in spans.iter_mut() {
                    let span_z_end = *span_z_end as u32;

                    if *voxel_type == type_from {
                        while influx_span.end <= span_z_start {
                            influx_span = if let Some(x) = influx_span_it.next() {
                                x
                            } else {
                                break 'fill_loop;
                            };
                        }

                        if span_z_end > influx_span.start {
                            // Fill this span.
                            *voxel_type = type_to;

                            // Propagate to adjacent rows
                            for &[dx, dy] in DIRS.iter() {
                                let adj_x = row_x.wrapping_add(dx);
                                let adj_y = row_y.wrapping_add(dy);
                                let z = span_z_start..span_z_end;
                                if adj_x < tile_size.x && adj_y < tile_size.y {
                                    // In the same tile - push to the local queue
                                    let head =
                                        &mut influx_spans_head[[adj_x as usize, adj_y as usize]];
                                    if head.is_none() {
                                        row_queue.push_back([adj_x, adj_y]);
                                    }

                                    let ptr =
                                        influx_spans.allocate(LocalPendingSpan { z, next: *head });
                                    *head = Some(ptr);
                                } else {
                                    // In the adjacent tile - push to the tile's influx queue later
                                    outflux_spans.push((adj_x, adj_y, z));
                                }
                            }
                        }
                    }

                    span_z_start = span_z_end;
                }

                flattened_spans.clear();
            }

            // Outflux into the adjacent tiles
            for (x, y, z) in outflux_spans.drain(..) {
                let adj_tile_x = tile_id[0].wrapping_add(((x as i32) >> tile_size_bits) as u32);
                let adj_tile_y = tile_id[1].wrapping_add(((y as i32) >> tile_size_bits) as u32);
                if adj_tile_x >= initial_domain.tile_count.x
                    || adj_tile_y >= initial_domain.tile_count.y
                {
                    // Out of bounds
                    continue;
                }
                debug_assert!(adj_tile_x != tile_id[0] || adj_tile_y != tile_id[1]);

                let entry = state.pending_tiles.entry([adj_tile_x, adj_tile_y]);
                if let Entry::Vacant(_) = entry {
                    state.tile_queue.push_back([adj_tile_x, adj_tile_y]);
                }

                let tile_state = entry.or_default();
                tile_state
                    .influx_spans
                    .push((x % tile_size.x, y % tile_size.y, z));
            }
        }
    }

    fn prefetch_tile(&self, tile: [u32; 2]) {
        let tile = &self.tiles[[tile[0] as usize, tile[1] as usize]];
        self.rle_store.prefetch_page(&[tile.rle_page_id]);
        self.rle_index_store
            .prefetch_page(&[tile.rle_index_page_id]);
    }
}

struct RangeFlattener<T> {
    endpoints: Vec<(T, i32)>,
}

impl<T: Ord + Clone + Eq> RangeFlattener<T> {
    fn with_capacity(i: usize) -> Self {
        Self {
            endpoints: Vec::with_capacity(i),
        }
    }

    fn clear(&mut self) {
        self.endpoints.clear();
    }

    fn insert(&mut self, range: Range<T>) {
        self.endpoints.push((range.start, 1));
        self.endpoints.push((range.end, -1));
    }

    /// Take a set of ranges (defined by calls to `insert`), merge overlapping
    /// ranges by taking their union, and produce a new set of ranges which have
    /// no overlap and are sorted by their locations.
    fn get_union(&mut self, mut range_handler: impl FnMut(Range<T>)) {
        let endpoints = &mut self.endpoints;
        endpoints.sort_by_key(|e| e.0.clone());

        let mut i = 0;
        let mut state = 0i32;
        let mut start = None;

        while i < endpoints.len() {
            // Track the nesting level
            let ep_x = endpoints[i].0.clone();
            let mut new_state = state;
            while i < endpoints.len() && endpoints[i].0 == ep_x {
                new_state += endpoints[i].1;
                i += 1;
            }

            if state == 0 {
                if new_state != 0 {
                    start = Some(ep_x);
                }
            } else if new_state == 0 {
                range_handler(start.take().unwrap()..ep_x);
            }

            state = new_state;
        }

        debug_assert_eq!(state, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_flattener_sanity() {
        let mut f = RangeFlattener::with_capacity(4);
        f.insert(7..11);
        f.insert(3..8);
        f.insert(13..15);
        f.insert(15..16);
        f.insert(20..24);

        let mut ranges = Vec::new();
        f.get_union(|r| ranges.push(r));

        assert_eq!(&ranges[..], &[3..11, 13..16, 20..24][..]);
    }
}
