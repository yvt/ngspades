//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Matrix4;
use std::ops::Range;

use crate::{mipbeamcast::mipbeamcast, terrain::Terrain};

/// Perfom a beam casting and create a 1D depth image.
///
/// `skip_buffer` is a temporary buffer that must have `output_depth.len() + 1`
/// elements. They don't have to be initialized.
pub fn opticast(
    terrain: &Terrain,
    azimuth: Range<f32>,
    projection: Matrix4<f32>,
    output_depth: &mut [f32],
    skip_buffer: &mut [u32],
) {
    assert!(skip_buffer.len() == output_depth.len() + 1);
    if output_depth.len() == 0 {
        return;
    }

    // Skip buffer would overflow if `output_depth` is too large
    assert!(
        output_depth.len() <= 0x20000000,
        "beam depth buffer is too large"
    );

    // TODO
}
