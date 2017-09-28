//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! A utility to generate a terrain data from a heightmap.
use cgmath::{Vector2, Vector3};
use byteorder::{WriteBytesExt, LittleEndian};
use std::cmp::min;
use std::io::{Cursor, Write};

use {Terrain, ColoredVoxel};

/// Provides an access to a heightmap from `HeightmapToTerrain`.
pub trait Heightmap {
    /// Retrieve the desired size of the created `Terrain`.
    fn size(&self) -> Vector3<usize>;

    /// Retrieve the altitude and the color at the specified position.
    fn get(&self, at: Vector2<usize>) -> (usize, ColoredVoxel<[u8; 4]>);
}

/// A utility to generate a terrain data from a heightmap.
pub struct HeightmapToTerrain<'a, T: 'a> {
    heightmap: &'a T,
}

impl<'a, T: Heightmap + 'a> HeightmapToTerrain<'a, T> {
    /// Constructs a `HeightmapToTerrain`.
    pub fn new(map: &'a T) -> Self {
        Self { heightmap: map }
    }

    /// Contructs a `Terrain`.
    pub fn build(&self) -> Terrain {
        let map = self.heightmap;
        let size = map.size();
        let mut t = Terrain::new(size);
        for y in 0..size.y {
            let y1 = if y == 0 { size.y - 1 } else { y - 1 };
            let y2 = if y + 1 == size.y { 0 } else { y + 1 };
            for x in 0..size.x {
                let x1 = if x == 0 { size.x - 1 } else { x - 1 };
                let x2 = if x + 1 == size.x { 0 } else { x + 1 };
                let (alt, color) = map.get(Vector2::new(x, y));
                let n1 = map.get(Vector2::new(x1, y)).0;
                let n2 = map.get(Vector2::new(x2, y)).0;
                let n3 = map.get(Vector2::new(x, y1)).0;
                let n4 = map.get(Vector2::new(x, y2)).0;
                assert!(alt >= 1, "The altitude must be at least 1.");
                assert!(
                    alt <= size.z,
                    "The altitude must be less than or equal to the terrain depth."
                );

                let mut min_neigh = min(min(min(n1, n2), min(n3, n4)), alt - 1);
                let mut cursor = Cursor::new(Vec::new());

                cursor.write_u16::<LittleEndian>(0).unwrap(); // empty voxels

                if min_neigh >= 2 {
                    cursor.write_u16::<LittleEndian>(1).unwrap(); // solid colored voxels
                    cursor.write(color.get_inner_ref()).unwrap();
                    cursor
                        .write_u16::<LittleEndian>((min_neigh - 1) as u16)
                        .unwrap(); // solid uncolored voxels
                } else {
                    min_neigh = 0;
                }

                cursor
                    .write_u16::<LittleEndian>((alt - min_neigh) as u16)
                    .unwrap(); // solid colored voxels
                for _ in 0..alt - min_neigh {
                    cursor.write(color.get_inner_ref()).unwrap();
                }

                cursor.write_u16::<LittleEndian>(0).unwrap(); // terminator

                *t.get_row_mut(Vector2::new(x, y)).unwrap().into_inner() = cursor.into_inner();
            }
        }
        t
    }
}
