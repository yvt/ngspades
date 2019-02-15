//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use alt_fp::FloatOrdSet;
use cgmath::{prelude::*, vec2, vec3, Vector2};
use ndarray::Array2;
use std::collections::HashMap;

use super::{BinnedGeometry, InitialDomain, Polygon};
use crate::mempool::{MemPageRefExt, MemPool};

impl BinnedGeometry {
    /// Construct an empty `BinnedGeometry`.
    pub fn new(pool: &impl MemPool, initial_domain: &InitialDomain) -> Self {
        let tile_count = initial_domain.tile_count;
        Self {
            tiles: Array2::default([tile_count.x as usize, tile_count.y as usize]),
            polygon_store: pool.new_store(),
        }
    }
}

const POLYGONS_PER_PAGE: usize = 65536 / std::mem::size_of::<Polygon>();

/// A buffer used for insertion of polygons into [`BinnedGeometry`].
///
/// `PolygonBinner` buffers polygons and when it becomes full it flushes the
/// buffered polygons to `BinnedGeometry`. The rationale of this design is
/// twofold: Firstly, The backing store ([`crate::mempool::MemStore`])
/// necessitates an explicit lock before accessing the contents. By buffering
/// the input, `PolygonBinner` reduces the number of lock operations.
/// Another reason is to make multi-threading more efficient. Contentions
/// between threads are minimized by limiting the duration and the frequency of
/// critical sections.
#[derive(Debug)]
pub struct PolygonBinner<T> {
    set: T,
    num_polygons: usize,
    max_polygons: usize,
    tile_size: Vector2<f32>,
    tile_size_inv: Vector2<f32>,
    tile_count: Vector2<f32>,
    tiles: HashMap<[usize; 2], Vec<Polygon>>,
}

impl<T> PolygonBinner<T> {
    /// Construct a `PolygonBinner`.
    ///
    /// `bin_set_source` is a `FnMut() -> impl DerefMut<Target = BinnedGeometry>`
    /// used to acquire a lock on `BinnedGeometry`.
    ///
    /// `capacity` specifies the number of polygons that can be buffered by
    /// the constructed `PolygonBinner`. The buffer is automatically flushed
    /// when the number of the stored buffer reaches `capacity`.
    pub fn new(capacity: usize, initial_domain: &InitialDomain, bin_set_source: T) -> Self {
        let tile_size = initial_domain.tile_size.cast::<f32>().unwrap();
        let tile_count = initial_domain.tile_count.cast::<f32>().unwrap() - vec2(1.0, 1.0);
        let tile_size_inv = vec2(1.0, 1.0).div_element_wise(tile_size);

        Self {
            set: bin_set_source,
            num_polygons: 0,
            max_polygons: capacity,
            tile_size,
            tile_size_inv,
            tile_count,
            tiles: HashMap::new(),
        }
    }
}

impl<T, S> PolygonBinner<T>
where
    T: FnMut() -> S,
    S: std::ops::DerefMut<Target = BinnedGeometry>,
{
    /// Flush the polygons in the buffer.
    pub fn flush(&mut self) {
        if self.tiles.len() == 0 {
            return;
        }

        let mut bin_set = (self.set)();
        let bin_set = &mut *bin_set; // enable split borrow

        let polygon_store = &bin_set.polygon_store;

        for (tile_coords, polygons) in self.tiles.drain() {
            let polygon_page_ids = &mut bin_set.tiles[tile_coords].polygon_page_ids;

            if polygon_page_ids.len() == 0 {
                polygon_page_ids.push(polygon_store.new_page(POLYGONS_PER_PAGE));
            }

            let mut page = polygon_store
                .get_page(*polygon_page_ids.last().unwrap())
                .write();
            for polygon in polygons {
                if page.as_vec().len() == page.as_vec().capacity() {
                    // Full - create a new page
                    let page_id = polygon_store.new_page(POLYGONS_PER_PAGE);
                    polygon_page_ids.push(page_id);

                    drop(page);
                    page = polygon_store.get_page(page_id).write();
                }

                page.as_vec().push(polygon);
            }
        }
    }

    /// Insert a polygon.
    pub fn insert(&mut self, polygon: Polygon) {
        use array::Array3;

        let tile_count = self.tile_count;
        let tile_size = self.tile_size;
        let tile_size_inv = self.tile_size_inv;

        let x_min = polygon.map(|p| p.x * tile_size_inv.x).fmin();
        let y_min = polygon.map(|p| p.y * tile_size_inv.y).fmin();
        let x_max = polygon.map(|p| p.x * tile_size_inv.x).fmax();
        let y_max = polygon.map(|p| p.y * tile_size_inv.y).fmax();

        let x_min = [x_min, 0.0].fmax() as i32;
        let y_min = [y_min, 0.0].fmax() as i32;
        let x_max = [x_max, tile_count.x].fmin() as i32;
        let y_max = [y_max, tile_count.y].fmin() as i32;

        if x_min > x_max || y_min > y_max {
            return;
        }

        let (x_min, y_min) = (x_min as usize, y_min as usize);
        let (x_max, y_max) = (x_max as usize, y_max as usize);

        if self.num_polygons + (x_max - x_min + 1) * (y_max - y_min + 1) > self.max_polygons {
            self.flush();
        }

        for x in x_min..=x_max {
            for y in y_min..=y_max {
                let polys = self.tiles.entry([x, y]).or_default();
                let origin = vec3((x as f32) * tile_size.x, (y as f32) * tile_size.y, 0.0);
                polys.push(polygon.map(|p| p - origin));
                self.num_polygons += 1;
            }
        }
    }
}
