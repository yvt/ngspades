//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate test;

use cgmath::{Vector2, Vector3};

use self::test::Bencher;
use super::*;

struct TestHeightmap;

impl heightmap::Heightmap for TestHeightmap {
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

#[bench]
fn bench_raytrace(b: &mut Bencher) {
    let t = heightmap::build_terrain_from_heightmap(&TestHeightmap);

    b.iter(move || {
        raytrace::raytrace(
            &t,
            Vector3::new(0.0, 0.0, 16.0),
            Vector3::new(64.0, 64.0, 0.0),
        )
    });
}
