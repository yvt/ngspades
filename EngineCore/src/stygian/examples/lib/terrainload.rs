//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{vec2, vec3};
use lazy_static::lazy_static;
use std::{
    ffi::OsStr,
    fs::File,
    io::{BufReader, Cursor},
    path::Path,
};

pub fn load_terrain(input_path: impl AsRef<Path>) -> ngsterrain::Terrain {
    let input_path: &Path = input_path.as_ref();

    let file = File::open(input_path).unwrap();
    let mut reader = BufReader::new(file);

    let mut terrain = if input_path.extension().map(OsStr::to_str) == Some(Some("vxl")) {
        ngsterrain::io::from_voxlap_vxl(vec3(512, 512, 64), &mut reader).unwrap()
    } else {
        ngsterrain::io::from_magicavoxel(&mut reader).unwrap()
    };

    // Pad the input to make the dimensions powers of two
    pad_terrain(&mut terrain);

    terrain.validate().unwrap();

    terrain
}

/// Pad a given `Terrain` to make the dimensions powers of two.
fn pad_terrain(terrain: &mut ngsterrain::Terrain) {
    let size = terrain.size();
    if !size.x.is_power_of_two() || !size.y.is_power_of_two() || !size.z.is_power_of_two() {
        let mut new_terrain = ngsterrain::Terrain::new(vec3(
            size.x.next_power_of_two(),
            size.y.next_power_of_two(),
            size.z.next_power_of_two(),
        ));

        println!(
            "The terrain size ({:?}) is not compliant; padding it to {:?}...",
            size,
            new_terrain.size()
        );

        for x in 0..size.x {
            for y in 0..size.y {
                (*new_terrain.get_row_mut(vec2(x, y)).unwrap().into_inner())
                    .clone_from(terrain.get_row(vec2(x, y)).unwrap().into_inner());
            }
        }
        *terrain = new_terrain;
    }
}

lazy_static! {
    pub static ref DERBY_RACERS: ngsterrain::Terrain = {
        let bytes = &include_bytes!("../../../ngsterrain/examples/vox/Derby Racers.vox")[..];
        let mut terrain = ngsterrain::io::from_magicavoxel(&mut Cursor::new(bytes)).unwrap();
        pad_terrain(&mut terrain);
        terrain
    };
}
