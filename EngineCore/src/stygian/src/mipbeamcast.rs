//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{vec2, Vector2};
use std::{
    cmp::{max, min},
    mem::swap,
};

use crate::utils::float::FloatSetExt;

/// Describes a single instance of incidence between a beam and a solid cell in
/// a 2D bitmap. Generated by `mipbeamcast`.
#[derive(Debug, Clone, Copy)]
pub struct MbcIncidence {
    pub cell_raw: MbcCell,
    /// The coordinates of the points where the beam entered (`intersections_raw[0]`)
    /// and left (`intersections_raw[1]`) the cell.
    ///
    /// The X and Y coordinates are the distances from the right and bottom
    /// edges of the cell, respectively. However, `intersections_raw[n][1].y`
    /// represent the distances from the top edge instead if
    /// `MbcInputPreproc::slope2_neg` is `true`.
    ///
    /// The values are represented in the `s?.F` fixed-point format.
    pub intersections_raw: [[Vector2<i32>; 2]; 2],
}

/// Represents a cell.
///
/// The cell described by `MbcCell` occupies
/// `x ∈ [pos.x, (pos.x + 1)) ∧ y ∈ [pos.y, (pos.y + 1))` (if `mip == 0`) or.
/// `x ∈ [pos.x << (mip - 1), (pos.x + 2) << (mip - 1)) ∧ y ∈ [pos.y << (mip - 1), (pos.y + 2) << (mip - 1))` (otherwise).
#[derive(Debug, Clone, Copy)]
pub struct MbcCell {
    pub pos: Vector2<i32>,
    /// The mip level. `0` represents the level 1. The base level (level 0)
    /// is not used.
    pub mip: u32,
}

impl MbcCell {
    /// The inclusive coordinates of the top-left corner of the region
    /// represented by `self`.
    pub fn pos_min(&self) -> Vector2<i32> {
        vec2(self.pos.x << self.mip, self.pos.y << self.mip)
    }

    /// The exclusive coordinates of the bottom-right corner of the region
    /// represented by `self`.
    pub fn pos_max(&self) -> Vector2<i32> {
        self.pos_min() + vec2(self.size(), self.size())
    }

    pub fn size(&self) -> i32 {
        2 << self.mip
    }
}

/// Describes preprocessing done to inputs and what should be done to
/// apply a reverse transformation on outputs to cancel out the effect of the
/// preprocessing.
#[derive(Debug, Clone, Copy)]
pub struct MbcInputPreproc {
    pub swap_xy: bool,
    pub flip_x: bool,
    pub flip_y: bool,
    pub size: Vector2<u32>,
    pub slope2_neg: bool,
}

impl MbcIncidence {
    pub fn cell(&self, preproc: &MbcInputPreproc) -> MbcCell {
        let mut cell = self.cell_raw;
        if preproc.flip_x {
            cell.pos.x = (preproc.size.x as i32 >> cell.mip) - 2 - cell.pos.x;
        }
        if preproc.flip_y {
            cell.pos.y = (preproc.size.y as i32 >> cell.mip) - 2 - cell.pos.y;
        }
        if preproc.swap_xy {
            swap(&mut cell.pos.x, &mut cell.pos.y);
        }
        cell
    }
}

/// Implements beam-casting on a 2D bitmap with adaptive mipmapping.
///
/// The size of the bitmap is defined by `size`.
///
/// A beam is defined as an intersection of two half planes or as the region
/// formed between two half lines connected at their endpoints. `start`
/// specifies the location of the beam's vertex. `dir1` and `dir2` specify
/// the directions of the edges extending from the vertex. The angle
/// of the beam must be much less than 45°.
///
/// `incidence_handler` is called for each discovered cell that entirely blocks
/// the beam. `MbcInputPreproc` (possibly wrapped by `T` by `preproc_filter`)
/// passed to the closure can be used to decode `MbcIncidence` on-the-fly, but
/// another option is to do it later using the `MbcInputPreproc` returned by
/// this function (all `MbcInputPreproc`s are equal).
///
/// `preproc_filter` is called once to perform pre-computations based on
/// `MbcInputPreproc` for the caller.
///
/// `num_mip_levels` must be equal to `log2(min(size.x, size.y)) + 1`.
pub fn mipbeamcast<T>(
    mut size: Vector2<u32>,
    num_mip_levels: u32,
    mut start: Vector2<f32>,
    mut dir1: Vector2<f32>,
    mut dir2: Vector2<f32>,
    preproc_filter: impl FnOnce(MbcInputPreproc) -> T,
    mut incidence_handler: impl FnMut(&MbcIncidence, &mut T),
) -> T {
    // Axis normalization
    let swap_xy = dir1.y.abs() > dir1.x.abs();
    if swap_xy {
        swap(&mut size.x, &mut size.y);
        swap(&mut start.x, &mut start.y);
        swap(&mut dir1.x, &mut dir1.y);
        swap(&mut dir2.x, &mut dir2.y);
    }

    let flip_x = dir1.x < 0.0;
    if flip_x {
        start.x = size.x as f32 - start.x;
        dir1.x = -dir1.x;
        dir2.x = -dir2.x;
    }

    let flip_y = dir1.y < 0.0;
    if flip_y {
        start.y = size.y as f32 - start.y;
        dir1.y = -dir1.y;
        dir2.y = -dir2.y;
    }

    let slope2_neg = dir2.y < 0.0;

    let preproc = MbcInputPreproc {
        size,
        swap_xy,
        flip_x,
        flip_y,
        slope2_neg,
    };

    let mut custom_preproc = preproc_filter(preproc);

    // `dir1` must be in the SE-Right octant
    debug_assert!(dir1.x >= 0.0, "{:?}", (dir1, dir2));
    debug_assert!(dir1.y >= 0.0, "{:?}", (dir1, dir2));
    debug_assert!(dir1.y <= dir1.x, "{:?}", (dir1, dir2));

    // `dir2` must be in one of the three octants at this point
    debug_assert!(dir2.x >= 0.0, "{:?}", (dir1, dir2));
    debug_assert!(dir2.x + dir2.y >= 0.0, "{:?}", (dir1, dir2));

    // Rescale `dir1` and `dir2`
    dir1 = vec2(1.0, dir1.y / dir1.x);
    dir2 = vec2(1.0, dir2.y / dir2.x);

    // Find the first cell.
    let mut cell;
    if start.x >= size.x as f32 || start.y >= size.y as f32 {
        // Never or only partly conincides with the map
        return custom_preproc;
    } else if start.y >= 0.0 {
        if start.x >= 0.0 {
            // Starts inside the map
            cell = MbcCell {
                pos: start.cast::<i32>().unwrap(),
                mip: 0,
            };
        } else {
            // Intercepts
            let y1 = start.y - start.x * dir1.y;
            let y2 = start.y - start.x * dir2.y;
            let (y1, y2) = ([y1, y2].min(), [y1, y2].max());
            if y1 >= 0.0 && y2 < size.y as f32 {
                // The beam enters the map from the left side
                cell = aabb_to_cell(0, y1 as i32, 0, y2 as i32);
            } else {
                // Only partly conincides with the map
                return custom_preproc;
            }
        }
    } else {
        // start.y < 0

        if dir2.y <= 0.0 {
            // Only partly conincides with the map
            return custom_preproc;
        }

        // Slopes of the half lines
        let (s1, s2) = ([dir1.y, dir2.y].min(), [dir1.y, dir2.y].max());

        // Intercepts
        let y1 = start.y + (size.x as f32 - start.x) * s1; // at `x = size.x`
        let y2 = start.y - start.x * s2; // at `x = 0`

        if y1 <= 0.0 {
            // Never or only partly conincides with the map
            return custom_preproc;
        }

        // Intercepts
        let x1 = start.x - start.y / s1; // at `y = 0`
        let x2 = start.x - start.y / s2; // at `y = 0`
        debug_assert!(x2 <= x1);

        if start.x >= 0.0 {
            // The beam enters the map from the top side
            cell = aabb_to_cell(x2 as i32, 0, x1 as i32, 0);
        } else if y2 > size.y as f32 {
            // Only partly conincides with the map
            return custom_preproc;
        } else {
            if x2 < 0.0 {
                // The beam enters the map from the top and left sides
                cell = aabb_to_cell(0, 0, x1 as i32, y2 as i32);
            } else {
                // The beam enters the map from the top side
                cell = aabb_to_cell(x2 as i32, 0, x1 as i32, 0);
            }
        }
    }

    if cell.mip >= num_mip_levels - 1
        || (cell.pos.x as u32) >= size.x - 1 >> cell.mip
        || (cell.pos.y as u32) >= size.y - 1 >> cell.mip
    {
        return custom_preproc;
    }

    // Convert the coordinates to s?.F fixed-point
    let start = (start * F_FAC_F).cast::<i32>().unwrap();
    let slope1 = (dir1.y * F_FAC_F) as i32;
    let slope2 = (dir2.y.abs() * F_FAC_F) as i32;

    debug_assert!(slope1 >= 0);
    debug_assert!(slope2 >= 0);

    let islope1 = ((1i64 << (F * 2)) / max(slope1, 1) as i64) as i32;
    let islope2 = ((1i64 << (F * 2)) / max(slope2, 1) as i64) as i32;

    // Thera are two moving points. Both of them start at `start` and moves
    // toward `dir1` and `dir2`.

    // Distance to the right border of the current cell from each current point
    let mut dx1 = (cell.pos_max().x << F) - start.x;
    let mut dx2 = dx1;

    // Distance to the bottom/top border of the current cell from each current
    // point. It's the top border iff the corresponding `slopeX` is negative.
    let mut dy1 = (cell.pos_max().y << F) - start.y;
    let mut dy2 = if slope2_neg {
        (2 << (cell.mip + F)) - dy1
    } else {
        dy1
    };

    loop {
        debug_assert!(dx1 >= 0, "{:?}", (size, num_mip_levels, start, dir1, dir2));
        debug_assert!(dy1 >= 0, "{:?}", (size, num_mip_levels, start, dir1, dir2));
        debug_assert!(dx2 >= 0, "{:?}", (size, num_mip_levels, start, dir1, dir2));
        debug_assert!(dy2 >= 0, "{:?}", (size, num_mip_levels, start, dir1, dir2));

        // Find the portal (the edge to exit the current cell through).
        // A portal is a polyline and (by definiton) has two endpoints.
        // Its AABB can be determined uniquely from the endpoints unless
        // it's shaped like "コ".
        let new_dy1 = dy1 - fix_mul(slope1, dx1);
        let new_dy2 = dy2 - fix_mul(slope2, dx2);
        let mut portal_x1;
        let portal_x2;
        let portal_y1;
        let portal_y2;
        let top_border = cell.pos_min().y - 1;
        let top = cell.pos_min().y;
        let bottom = cell.pos_max().y;
        let right = cell.pos_max().x;

        let last_intersections = [vec2(dx1, dy1), vec2(dx2, dy2)];

        if new_dy1 < 0 {
            // Bottom
            portal_y1 = bottom;
            portal_x1 = right - fix2int_ceil(dx1 - fix_mul(dy1, islope1));

            dx1 -= fix_mul(dy1, islope1);
            dy1 = 0;
        } else {
            // Right
            portal_y1 = bottom - fix2int_ceil(new_dy1);
            portal_x1 = right;

            dx1 = 0;
            dy1 = new_dy1;
        }

        if new_dy2 < 0 {
            if slope2_neg {
                // Top
                portal_y2 = top_border;

                // The portal includes the right edge. Make sure to take it into account
                portal_x1 = max(portal_x1, right);
            } else {
                // Bottom
                portal_y2 = bottom;
            }
            portal_x2 = right - fix2int_ceil(dx2 - fix_mul(dy2, islope2));

            dx2 -= fix_mul(dy2, islope2);
            dy2 = 0;
        } else {
            // Right
            if slope2_neg {
                portal_y2 = top + fix2int_floor(new_dy2);
            } else {
                portal_y2 = bottom - fix2int_ceil(new_dy2);
            }
            portal_x2 = right;

            dx2 = 0;
            dy2 = new_dy2;
        }

        // Find the next cell that includes the entirety of the portal
        let new_cell = aabb_to_cell(
            min(portal_x1, portal_x2),
            min(portal_y1, portal_y2),
            max(portal_x1, portal_x2),
            max(portal_y1, portal_y2),
        );

        // Eureka!
        incidence_handler(
            &MbcIncidence {
                cell_raw: cell,
                intersections_raw: [last_intersections, [vec2(dx1, dy1), vec2(dx2, dy2)]],
            },
            &mut custom_preproc,
        );

        if new_cell.mip >= num_mip_levels - 1
            || (new_cell.pos.x as u32) >= size.x - 1 >> new_cell.mip
            || (new_cell.pos.y as u32) >= size.y - 1 >> new_cell.mip
        {
            return custom_preproc;
        }

        // Calculate the displacement and adjust the state variables
        let dx = new_cell.pos_max().x - cell.pos_max().x;
        dx1 += dx << F;
        dx2 += dx << F;

        dy1 += (new_cell.pos_max().y - cell.pos_max().y) << F;
        if slope2_neg {
            dy2 -= (new_cell.pos_min().y - cell.pos_min().y) << F;
        } else {
            dy2 += (new_cell.pos_max().y - cell.pos_max().y) << F;
        }

        cell = new_cell;
    }
}

/// Find the smallest cell that includes a specified rectangular region.
/// All endpoints are inclusive.
fn aabb_to_cell(x_min: i32, y_min: i32, x_max: i32, y_max: i32) -> MbcCell {
    debug_assert!(x_min <= x_max);
    debug_assert!(y_min <= y_max);

    // `ceil(log2( max(y_max - y_min, x_max - x_min) + 1 ))`
    let mip_level = 31 - ((x_max - x_min) | (y_max - y_min) | 1).leading_zeros();

    let cell = {
        // It might be `mip_level` or `mip_level + 1`
        let x_min_rnd = x_min >> mip_level;
        let y_min_rnd = y_min >> mip_level;
        let x_max_rnd = (x_max - (1 << mip_level)) >> mip_level;
        let y_max_rnd = (y_max - (1 << mip_level)) >> mip_level;

        if x_min_rnd >= x_max_rnd && y_min_rnd >= y_max_rnd {
            MbcCell {
                pos: vec2(x_min_rnd, y_min_rnd),
                mip: mip_level,
            }
        } else {
            MbcCell {
                pos: vec2(x_min_rnd >> 1, y_min_rnd >> 1),
                mip: mip_level + 1,
            }
        }
    };

    debug_assert!(
        cell.pos_min().x <= x_min
            && cell.pos_min().y <= y_min
            && cell.pos_max().x >= x_max
            && cell.pos_max().y >= y_max,
        "{:?}",
        (x_min, y_min, x_max, y_max, mip_level, cell)
    );

    cell
}

pub const F: u32 = 16;
pub const F_FAC_F: f32 = (1 << F) as f32;

fn fix_mul(x: i32, y: i32) -> i32 {
    (((x as i64) * (y as i64)) >> F) as i32
}

fn fix2int_floor(x: i32) -> i32 {
    x >> F
}

fn fix2int_ceil(x: i32) -> i32 {
    (x + (1 << F) - 1) >> F
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity() {
        let mut patterns = vec![
            [vec2(0.5, 0.5), vec2(1.0, 0.6), vec2(1.0, 0.9)],
            [vec2(-0.5, -0.5), vec2(1.0, 0.6), vec2(1.0, 0.9)],
            [vec2(-0.5, -0.5), vec2(1.0, -0.1), vec2(1.0, 0.1)],
            [vec2(-0.5, 16.5), vec2(1.0, -0.4), vec2(1.0, -0.2)],
        ];

        use array::Array3;
        use cgmath::{Deg, Matrix2};
        for i in 0..360 {
            let m = Matrix2::from_angle(Deg(i as f32));
            let mut verts = [vec2(-20.0, 0.0), vec2(1.0, -0.2), vec2(1.0, 0.2)];
            verts = verts.map(|v| m * v);
            verts[0] += vec2(8.0, 8.0);
            patterns.push(verts);
        }

        for [start, dir1, dir2] in patterns {
            dbg!((start, dir1, dir2));
            mipbeamcast(
                vec2(16, 16),
                5,
                start,
                dir1,
                dir2,
                |x| x,
                |incidence, preproc| {
                    let cell = incidence.cell(preproc);
                    dbg!(cell);
                    println!("{:?} - {:?}", cell.pos_min(), cell.pos_max());
                },
            );
        }
    }

    #[test]
    fn sanity2() {
        let patterns = vec![
            [
                vec2(256.0, 256.0),
                vec2(1.0, 0.936591),
                vec2(1.0, 0.9071967),
            ],
            [
                vec2(256.0, 256.0),
                vec2(1.0, 0.87908727),
                vec2(1.0, 0.8506685),
            ],
            [
                vec2(256.8530883, 256.25552368),
                vec2(1.0, 0.021339143),
                vec2(1.0, -0.00000017484555),
            ],
        ];
        for [start, dir1, dir2] in patterns {
            dbg!((start, dir1, dir2));
            mipbeamcast(
                vec2(512, 512),
                10,
                start,
                dir1,
                dir2,
                |x| x,
                |incidence, preproc| {
                    let cell = incidence.cell(preproc);
                    dbg!(cell);
                    println!("{:?} - {:?}", cell.pos_min(), cell.pos_max());
                },
            );
        }
    }
}
