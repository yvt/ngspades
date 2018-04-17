//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngsterrain;

use ngsterrain::*;
use cgmath::Vector3;

#[test]
fn create_terrain() {
    let t = Terrain::new(Vector3::new(64, 64, 16));
    if let Err((coord, err)) = t.validate() {
        panic!("At row ({:?}; {:?}): {}", coord, t.get_row(coord), err);
    }
}
