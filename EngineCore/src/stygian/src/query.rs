//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use alt_fp::FloatOrdSet;
use cgmath::{Point3, Vector4};

use crate::depthimage::DepthImage;

/// Stores pre-calculated parameters for occulusion query.
#[derive(Debug)]
pub struct QueryContext {}

impl QueryContext {
    pub fn new(_depth_image: &DepthImage) -> Self {
        unimplemented!()
    }

    pub fn query_vs_aabb(&self, _aabb: [Point3<f32>; 2]) {
        unimplemented!()
    }
}

/// Clip a given clip-space AABB in the W direction to the view volume (a.k.a.
/// the frustum).
///
/// This function is suitable for pre-perspective-division processing as it
/// removes the `w ≤ -0` part of an input AABB.
///
/// Returns `Some(x)` if the clipped AABB is not empty. The returned W
/// coordinates guaranteed to have no negative values (not even negative zero),
/// ensuring the result of perspective division is valid.
/// This function only modifies the lower bound of the W coordinate — the
/// returned `x` only differs from the input by the value of `x[0].w`.
///
/// `None` is returned if the input is completed clipped away.
///
/// # Examples
///
///     # use stygian::clip_w_cs_aabb;
///     # use cgmath::vec4;
///     // { (1, y, z, w) | 2≤y≤4 ∧ 2≤z≤3 } never intersects with
///     // { (x, y, z, W) | |x|≤2 ∧ |y|≤2 ∧ 0≤z≤2 } for any 0≤W≤1
///     assert!(dbg!(
///         clip_w_cs_aabb([
///             vec4(1.0, 2.0, 2.0, 0.0),
///             vec4(1.0, 4.0, 3.0, 1.0),
///         ])
///     ).is_none());
///
///     // But it does for every W≥2
///     assert_eq!(dbg!(
///         clip_w_cs_aabb([
///             vec4(1.0, 2.0, 2.0, 0.0),
///             vec4(1.0, 4.0, 3.0, 3.0),
///         ])
///     ).unwrap()[0].w, 2.0);
///
pub fn clip_w_cs_aabb(aabb: [Vector4<f32>; 2]) -> Option<[Vector4<f32>; 2]> {
    if aabb[1].z < 0.0 {
        return None;
    }

    // The minimum W slice of the W extrusion of `aabb` that intersects
    // with the view volume
    let min_w = [
        // `x = ±w`
        [aabb[0].x, -aabb[1].x].fmin(),
        // `y = ±w`
        [aabb[0].y, -aabb[1].y].fmin(),
        // `z = w`
        aabb[0].z,
        // The view volume is bounded by `w > 0`. This must be the last
        // element so that the clipped result doesn't have a negative zero.
        0.0,
    ].fmax();

    // If `aabb[1].w` has negative zero, it would be rejected here
    if min_w >= aabb[1].w {
        return None;
    }

    Some([
        // `min_w` must appear later in case `aabb[0].w` is negative zero
        aabb[0].truncate().extend([aabb[0].w, min_w].fmax()),
        aabb[1],
    ])
}
