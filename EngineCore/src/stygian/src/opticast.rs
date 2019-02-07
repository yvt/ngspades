//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{prelude::*, vec2, Matrix4, Point3};
use std::{cmp::max, ops::Range};

use crate::{
    debug::Trace,
    mipbeamcast::{mipbeamcast, F_FAC_F},
    terrain::Terrain,
    utils::float::FloatSetExt,
    DEPTH_FAR,
};

/// In a skip buffer, this flag indicates there are no more vacant elements
/// afterward.
///
/// Why use a flag instead of a specific value? Some of x86's status registers
/// update automatically based on MSB. See <https://godbolt.org/z/tqCzQC>
///
/// Why `u32`? x86 doesn't allow 16-bit registers for indexed addressing!
const EOB_BIT: u32 = 1 << 31;

/// Perfom a beam casting and create a 1D depth image.
///
/// `skip_buffer` is a temporary buffer that must have `output_depth.len() + 1`
/// elements. They don't have to be initialized.
#[inline(never)]
pub fn opticast(
    terrain: &Terrain,
    azimuth: Range<f32>,
    _inclination: Range<f32>,
    projection: Matrix4<f32>,
    lateral_projection: Matrix4<f32>,
    eye: Point3<f32>,
    output_depth: &mut [f32],
    skip_buffer: &mut [u32],
    trace: &mut impl Trace,
) {
    assert!(skip_buffer.len() == output_depth.len() + 1);
    if output_depth.len() == 0 {
        return;
    }

    // Skip buffer would overflow if `output_depth` is too large
    assert!(
        output_depth.len() <= 0x40000000,
        "beam depth buffer is too large"
    );

    // The skip buffer is used to implement the reverse painter's algorithm.
    // Clear the skip buffer
    for x in skip_buffer.iter_mut() {
        *x = 0;
    }
    skip_buffer[output_depth.len()] = EOB_BIT;

    // Prepare beam-casting
    let dir1 = vec2(azimuth.start.cos(), azimuth.start.sin());
    let dir2 = vec2(azimuth.end.cos(), azimuth.end.sin());
    let theta = (azimuth.start + azimuth.end) * 0.5;
    let dir_primary = vec2(theta.cos(), theta.sin());

    // Main loop
    mipbeamcast(
        terrain.size().truncate().cast().unwrap(),
        terrain.levels.len() as u32,
        vec2(eye.x, eye.y),
        dir1,
        dir2,
        |preproc| {
            let terrain_size = terrain.size().truncate();

            let mut local_dir_primary = dir_primary;
            let mut local_eye = eye;
            if preproc.swap_xy {
                std::mem::swap(&mut local_dir_primary.x, &mut local_dir_primary.y);
                std::mem::swap(&mut local_eye.x, &mut local_eye.y);
            }
            if preproc.flip_x {
                local_dir_primary.x = -local_dir_primary.x;
                local_eye.x = terrain_size.x as f32 - local_eye.x;
            }
            if preproc.flip_y {
                local_dir_primary.y = -local_dir_primary.y;
                local_eye.y = terrain_size.y as f32 - local_eye.y;
            }

            let local_eye_dist = vec2(local_eye.x, local_eye.y).dot(local_dir_primary);

            (preproc, local_dir_primary, local_eye_dist)
        },
        |incidence, &mut (preproc, local_dir_primary, local_eye_dist)| {
            // Localize captured variables. This does have an impact on the
            // generated assembly code.
            let output_depth = &mut output_depth[..];
            let skip_buffer = &mut skip_buffer[..];
            let (eye, projection) = (eye, projection);

            // TODO: Early-out by Z range

            // Get the row
            let cell = incidence.cell(&preproc);

            debug_assert!((cell.mip as usize) < terrain.levels.len());
            let level = unsafe { terrain.levels.get_unchecked(cell.mip as usize) };

            let level_size_bits_x = terrain.size_bits.x + 1 - max(cell.mip, 1);
            let row_index = cell.pos.x as usize + ((cell.pos.y as usize) << level_size_bits_x);
            if cell.mip > 0 {
                debug_assert!(cell.pos.x < (1 << terrain.size_bits.x + 1 - cell.mip) - 1);
                debug_assert!(cell.pos.y < (1 << terrain.size_bits.y + 1 - cell.mip) - 1);
            } else {
                debug_assert!(cell.pos.x < (1 << terrain.size_bits.x));
                debug_assert!(cell.pos.y < (1 << terrain.size_bits.y));
            }
            debug_assert!(cell.pos.x >= 0);
            debug_assert!(cell.pos.y >= 0);
            debug_assert!(row_index < level.rows.len());
            let row = unsafe { level.rows.get_unchecked(row_index) };

            // Find the left/right-most intersections
            use array::Array2;
            let intersections = incidence
                .intersections_raw
                .map(|x| x.map(|x| vec2(x.x as f32, x.y as f32) * (1.0 / F_FAC_F)));
            let cell_raw_pos_f = incidence.cell_raw.pos_min().cast::<f32>().unwrap();
            let cell_size = incidence.cell_raw.size();
            let cell_size_f = cell_size as f32;

            // `dot(x, dir_primary)` for each intersections of the beam and the cell
            let intersction_dists = intersections.map(|[i1, i2]| {
                [
                    local_dir_primary.dot(cell_raw_pos_f) - local_eye_dist
                        + local_dir_primary.y * cell_size_f
                        + local_dir_primary.x * cell_size_f
                        - i1.dot(local_dir_primary),
                    if preproc.slope2_neg {
                        local_dir_primary.dot(cell_raw_pos_f) - local_eye_dist
                            + local_dir_primary.x * cell_size_f
                            - (i2.x * local_dir_primary.x - i2.y * local_dir_primary.y)
                    } else {
                        local_dir_primary.dot(cell_raw_pos_f) - local_eye_dist
                            + local_dir_primary.y * cell_size_f
                            + local_dir_primary.x * cell_size_f
                            - (i2.x * local_dir_primary.x + i2.y * local_dir_primary.y)
                    },
                ]
            });
            let intersction_dists = [intersction_dists[0].max(), intersction_dists[1].min()];

            // Rasterize spans
            for span in row.iter() {
                // TODO: Calculations done here fail to be vectorized - figure
                //       out how to make it SIMD-friendly or use SIMD explicitly

                // Find the “reverse” AABB (like the incircle of a triangle)
                let bottom_above_eye = span.start as f32 > eye.z;
                let top_below_eye = (span.end as f32) < eye.z;
                let span_bottom_dist = intersction_dists[bottom_above_eye as usize];
                let span_top_dist = intersction_dists[top_below_eye as usize];

                let mut p1 = projection.w
                    + projection.x * span_bottom_dist
                    + projection.z * span.start as f32;
                let mut p2 =
                    projection.w + projection.x * span_top_dist + projection.z * span.end as f32;

                // Apply the lateral projection matrix.
                // The left and right edges have different Z values. The matrix
                // compensates for that.
                let p1_lat = lateral_projection.x * span_bottom_dist
                    + lateral_projection.z * span.start as f32;
                let p2_lat =
                    lateral_projection.x * span_top_dist + lateral_projection.z * span.end as f32;
                // D[(a + ct) / (b + dt), t = 0] = (bc - ad) / b²
                // Use this approximation to find the minimum Z value for each
                // of the top and bottom edges.
                p1.z -= (p1.z * p1_lat.w - p1.w * p1_lat.z).abs() * (1.0 / p1.w);
                p2.z -= (p2.z * p2_lat.w - p2.w * p2_lat.z).abs() * (1.0 / p2.w);

                // Clip the line segment by the plane `z == w` (near plane)
                let clip_states = [p1.z > p1.w, p2.z > p2.w];
                let clip_state = clip_states[0] as usize + clip_states[1] as usize;
                if clip_state == 2 {
                    // Completely clipped
                    continue;
                } else if clip_state == 1 {
                    // Partial clipped
                    let dot1 = p1.z - p1.w;
                    let dot2 = p2.z - p2.w;
                    let fraction = dot1 / (dot1 - dot2);
                    debug_assert!(fraction >= 0.0 && fraction <= 1.0);
                    let mut midpoint = p1.lerp(p2, fraction);
                    midpoint.w = midpoint.z;
                    if clip_states[0] {
                        p1 = midpoint;
                    } else {
                        p2 = midpoint;
                    }
                }

                // Rasterize the span
                let (mut p1, mut p2) = (Point3::from_homogeneous(p1), Point3::from_homogeneous(p2));
                trace.opticast_span(
                    cell.pos_min().cast().unwrap(),
                    1 << cell.mip,
                    span.start as u32..span.end as u32,
                );

                p1.y *= output_depth.len() as f32;
                p2.y *= output_depth.len() as f32;

                let y1 = [p1.y.ceil(), 0.0].max() as i32;
                let y2 = [p2.y, output_depth.len() as f32].min() as i32;
                if y1 >= y2 {
                    continue;
                }

                let (y1, y2) = (y1 as u32, y2 as u32);
                let delta_z = (p2.z - p1.z) / (p2.y - p1.y);
                let mut last_z = p1.z + delta_z * (y1 as f32 - p1.y);

                let mut i = y1;

                let end_skip = *unsafe { skip_buffer.get_unchecked(y2 as usize) };

                'draw: while (i & EOB_BIT) == 0 {
                    let skip = *unsafe { skip_buffer.get_unchecked(i as usize) };
                    if skip != 0 {
                        i += skip;
                        if i >= y2 {
                            break;
                        }
                        last_z += delta_z * skip as f32;
                        continue;
                    }

                    loop {
                        let next_z = last_z + delta_z;
                        *unsafe { output_depth.get_unchecked_mut(i as usize) } =
                            [last_z, next_z].min();
                        *unsafe { skip_buffer.get_unchecked_mut(i as usize) } = end_skip + (y2 - i);
                        i += 1;

                        if i >= y2 {
                            break 'draw;
                        }

                        last_z = next_z;

                        if *unsafe { skip_buffer.get_unchecked(i as usize) } != 0 {
                            break;
                        }
                    }
                }
            }
        },
    );

    // Draw sky
    {
        let mut i = 0;
        while (i & EOB_BIT) == 0 {
            let skip = *unsafe { skip_buffer.get_unchecked(i as usize) };
            if skip != 0 {
                i += skip;
                continue;
            }

            loop {
                *unsafe { output_depth.get_unchecked_mut(i as usize) } = DEPTH_FAR;
                i += 1;

                if *unsafe { skip_buffer.get_unchecked(i as usize) } != 0 {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opticast_single1() {
        use crate::terrainload::DERBY_RACERS;
        let terrain = Terrain::from_ngsterrain(&DERBY_RACERS).unwrap();
        let azimuth = -2.86935139..-2.85282278;
        let inclination = -0.575736046..0.370368004;
        let projection = Matrix4::new(
            0.0,
            0.517639935,
            -0.00174160628,
            0.86993289,
            0.0,
            0.252701819,
            -0.000962537655,
            0.480787873,
            0.0,
            0.797484278,
            0.000219854119,
            -0.109817199,
            0.0,
            -11.9622612,
            0.997703194,
            1.64726257,
        );
        let lateral_projection = Matrix4::zero();
        let eye = Point3::new(64.0, 64.0, 15.0);
        let mut output_depth = [0.0; 69];
        let mut skip_buffer = [0; 70];
        opticast(
            &terrain,
            azimuth,
            inclination,
            projection,
            lateral_projection,
            eye,
            &mut output_depth,
            &mut skip_buffer,
            &mut crate::NoTrace,
        );

        dbg!(&output_depth[..]);

        // Check for the incorrect output illustrated in the image:
        // <ipfs://QmPGxf4xRk8LxAoxyVWGk4czisRTXRbd7Dhi72qbYW5oGF>
        for &x in &output_depth[37..] {
            assert_eq!(x, 0.0);
        }
    }

    #[test]
    fn opticast_single2() {
        use crate::terrainload::DERBY_RACERS;
        let terrain = Terrain::from_ngsterrain(&DERBY_RACERS).unwrap();
        let azimuth = -2.58605146..-2.56464362;
        let inclination = -0.625534951..0.407851398;
        let projection = Matrix4::new(
            0.0,
            0.577354848,
            -0.00194229803,
            0.970178484,
            0.0,
            0.116483852,
            -0.0004326083,
            0.216087967,
            0.0,
            0.799330353,
            0.000219854119,
            -0.109817199,
            0.0,
            -11.9899483,
            0.997703194,
            1.64726257,
        );
        let lateral_projection = Matrix4::new(
            0.014344329,
            -0.00046780752,
            0.00000463083779,
            -0.00231310539,
            0.00319490628,
            0.00210033869,
            -0.0000207913035,
            0.0103852628,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        let eye = Point3::new(64.0, 64.0, 15.0);
        let mut output_depth = [0.0; 69];
        let mut skip_buffer = [0; 70];
        opticast(
            &terrain,
            azimuth,
            inclination,
            projection,
            lateral_projection,
            eye,
            &mut output_depth,
            &mut skip_buffer,
            &mut crate::NoTrace,
        );

        dbg!(&output_depth[..]);

        // Check for the incorrect output illustrated in the image:
        // <ipfs://QmTPFyLy76mrgWabCKsQzS15kZFXR7PjpC3mswxDmj3RyY>
        assert!(
            output_depth[41] <= 0.041894495,
            "{:?} <= 0.04189449",
            output_depth[41]
        );
    }
}
