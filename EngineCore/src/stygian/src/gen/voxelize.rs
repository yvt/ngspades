//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use alt_fp::FloatOrdSet;
use cgmath::{vec2, Vector3};
use ndarray::{s, Array3};

use super::{
    tri::tricrast,
    utils::{bitarray_enum_spans, bitarray_set_range, BITS},
    BinnedGeometry, InitialDomain, Polygon, Span, VoxelBitmap, VoxelBitmapTile, VoxelType,
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
