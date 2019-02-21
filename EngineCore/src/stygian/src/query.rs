//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use alt_fp::{fma, FloatOrdSet};
use cgmath::{Point3, Vector4};
use std::borrow::Borrow;

use crate::depthimage::DepthImage;

/// Stores pre-calculated parameters for performing occulusion query on a
/// rendered depth image.
#[derive(Debug)]
pub struct QueryContext<T> {
    depth_image: T,
    size_half: [f32; 2],
    size_inv: [f32; 2],
}

impl<T: Borrow<DepthImage>> QueryContext<T> {
    /// Construct a `QueryContext` for performing occlusion query on a given
    /// rendered depth image.
    ///
    /// `depth_image` can be anything that can borrow `DepthImage`.
    /// However, the dimensions ([`DepthImage::size`]) of the borrowed
    /// `DepthImage`s must stay consistent.
    pub fn new(depth_image: T) -> Self {
        let size = depth_image.borrow().size();

        Self {
            depth_image,
            size_half: [size.x as f32 * 0.5, size.y as f32 * 0.5],
            size_inv: [2.0 / size.x as f32, 2.0 / size.y as f32],
        }
    }

    /// Query whether a given viewport-space AABB is visible.
    ///
    /// The AABB is considered visible if any pixel in `DepthImage` overlapping
    /// with the AABB has a depth value less than the maximum Z coordinate of
    /// the AABB.
    ///
    /// Returns `false` if the AABB is completely occluded. Otherwise, `true`
    /// is returned.
    pub fn query_vs_aabb(&self, mut aabb: [Point3<f32>; 2]) -> bool {
        let depth_image = self.depth_image.borrow();
        let [img_w_half, img_h_half] = self.size_half;
        let [img_w, img_h]: [usize; 2] = depth_image.size().into();

        // Make sure `aabb` is not infinitely thin to handle the cases
        // where both of the minimum and maximum coordinates fall into the
        // same integer coordinate correctly
        aabb[1].x = [aabb[1].x, aabb[0].x + self.size_inv[0]].fmax();
        aabb[1].y = [aabb[1].y, aabb[0].y + self.size_inv[1]].fmax();

        // The distances of the rectangle's edges from the viewport border
        let x_min = fma![(aabb[0].x) * img_w_half + img_w_half];
        let x_max = fma![(aabb[1].x) * (-img_w_half) + img_w_half];
        let y_min = fma![(aabb[0].y) * img_h_half + img_h_half];
        let y_max = fma![(aabb[1].y) * (-img_h_half) + img_h_half];

        let (x_min, y_min) = ([x_min, 0.0].fmax(), [y_min, 0.0].fmax());
        let (x_max, y_max) = ([x_max, 0.0].fmax(), [y_max, 0.0].fmax());

        // Convert them to the relative positions from the top-left corner
        let (x_min, y_min) = (x_min as i32, y_min as i32);
        let (x_max, y_max) = (img_w as i32 - x_max as i32, img_h as i32 - y_max as i32);

        if x_min >= x_max || y_min >= y_max {
            return false;
        }

        let (x_min, y_min) = (x_min as usize, y_min as usize);
        let (x_max, y_max) = (x_max as usize, y_max as usize);

        let img = depth_image.pixels();

        debug_assert!(x_max <= img_w, "{:?} <= {:?}", x_max, img_w);
        debug_assert!(y_max <= img_h, "{:?} <= {:?}", y_max, img_h);

        for y in y_min..y_max {
            for x in x_min..x_max {
                let p = unsafe { img.get_unchecked(x + y * img_w) };
                if aabb[1].z >= *p {
                    return true;
                }
            }
        }

        false
    }

    /// Query whether a given clip-space AABB is visible.
    ///
    /// This method is a shortcut method that internally calls
    /// [`clip_w_cs_aabb`] and [`QueryContext::query_vs_aabb`].
    pub fn query_cs_aabb(&self, aabb: [Vector4<f32>; 2]) -> bool {
        if let Some(aabb) = clip_w_cs_aabb(aabb) {
            // FIXME: Reducing divide operations here might be beneficial
            let p0 = Point3::from_homogeneous(aabb[0]);
            let p1 = Point3::from_homogeneous(aabb[1]);
            let p2 = Point3::from_homogeneous(aabb[0].truncate().extend(aabb[1].w));
            let p3 = Point3::from_homogeneous(aabb[1].truncate().extend(aabb[0].w));
            let vs_aabb = [
                Point3::new(
                    [p0.x, p2.x].fmin(),
                    [p0.y, p2.y].fmin(),
                    [p0.z, p2.z].fmin(),
                ),
                Point3::new(
                    [p1.x, p3.x].fmax(),
                    [p1.y, p3.y].fmax(),
                    [p1.z, p3.z].fmax(),
                ),
            ];
            self.query_vs_aabb(vs_aabb)
        } else {
            false
        }
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
    ]
    .fmax();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_vs_aabb_sanity1() {
        let di = DepthImage::new([4, 4].into());
        let query = QueryContext::new(&di);

        // Visible
        assert!(query.query_vs_aabb([Point3::new(-0.5, -0.4, 0.2), Point3::new(0.2, 0.2, 0.5)]));

        // Y clip
        assert!(!query.query_vs_aabb([Point3::new(-0.5, 1.2, 0.2), Point3::new(0.2, 1.4, 0.5)]));

        // Z clip
        assert!(!query.query_vs_aabb([Point3::new(-0.5, -0.4, -0.5), Point3::new(0.2, 0.2, -0.3)]));

        // Infinitely thin AABBs
        assert!(query.query_vs_aabb([Point3::new(-0.5, 0.0, 0.2), Point3::new(0.2, 0.0, 0.5)]));
        assert!(query.query_vs_aabb([Point3::new(0.0, 0.0, 0.2), Point3::new(0.0, 0.0, 0.5)]));
    }

    #[test]
    fn query_vs_aabb_sanity2() {
        let mut di = DepthImage::new([4, 4].into());
        di.image[0] = 1.0; // [0, 0]

        let query = QueryContext::new(&di);

        // [0, 0]–[1, 1]
        assert!(query.query_vs_aabb([Point3::new(-1.0, -1.0, 0.2), Point3::new(-0.2, -0.2, 0.5)]));

        // [0, 0]–[0, 0]
        assert!(!query.query_vs_aabb([Point3::new(-1.0, -1.0, 0.2), Point3::new(-0.8, -0.8, 0.5)]));
        assert!(query.query_vs_aabb([Point3::new(-1.0, -1.0, 0.2), Point3::new(-0.8, -0.8, 1.1)]));
    }
}
