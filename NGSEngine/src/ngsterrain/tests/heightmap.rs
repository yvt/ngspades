//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngsterrain;

use ngsterrain::*;
use cgmath::{Vector2, Vector3};
use std::cmp;

use heightmap::Heightmap;

struct TestHeightmap;

impl Heightmap for TestHeightmap {
    fn size(&self) -> Vector3<usize> {
        Vector3::new(64, 64, 16)
    }

    fn get(&self, at: Vector2<usize>) -> (usize, ColoredVoxel<[u8; 4]>) {
        let vox = ColoredVoxel::from_values(
            [at.x as u8 + 64, at.y as u8 + 96, at.x as u8 + 128],
            at.y as u8 + 64,
        );
        (((at.x ^ at.y) & 15) + 1, vox)
    }
}

#[test]
fn create_from_heightmap() {
    let hm = TestHeightmap;
    let t = heightmap::HeightmapToTerrain::new(&hm).build();
    if let Err((coord, err)) = t.validate() {
        panic!("At row {:?}: {}", (coord, t.get_row(coord)), err);
    }
    for (coord, row) in t.rows() {
        let mut chunks = row.chunks();
        let mut height = 0;

        while let Some(chunk) = chunks.next() {
            for (voxels_z, voxels) in chunk {
                let num_voxels = voxels.num_voxels();
                height = cmp::max(height, voxels_z + num_voxels);
            }
        }

        assert_eq!(height, hm.get(coord).0, "At row {:?}", (coord, row));

        assert_eq!(
            t.get_voxel(coord.extend(hm.get(coord).0 - 1)),
            Some(SolidVoxel::Colored(hm.get(coord).1.as_ref())),
            "At row {:?}",
            (coord, row)
        );
    }
}

#[test]
fn raytrace_on_heightmap() {
    let hm = TestHeightmap;
    let t = heightmap::HeightmapToTerrain::new(&hm).build();
    for (coord, row) in t.rows() {
        match raytrace::raytrace(
            &t,
            Vector3::new(coord.x as f32 + 0.5, coord.y as f32 + 0.5, 64.0),
            Vector3::new(coord.x as f32 + 0.1, coord.y as f32 + 0.1, 0.0),
        ) {
            raytrace::RaytraceResult::Hit(hit) => {
                assert_eq!(hit.normal, CubeFace::PositiveZ, "at row {:?}", (coord, row));
                assert_eq!(
                    hit.voxel,
                    coord.extend(hm.get(coord).0 - 1),
                    "at row {:?}",
                    (coord, row)
                );
            }
            x => {
                panic!("Raytrace at the row {:?} failed: {:?}", (coord, row), &x);
            }
        }
    }
}
