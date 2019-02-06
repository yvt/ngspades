//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{vec2, vec4, Matrix4, Point3};
use std::{
    cmp::{max, min},
    f32::{consts::PI, INFINITY, NEG_INFINITY},
    ops::Range,
};

use crate::{
    debug::Trace, mipbeamcast::mipbeamcast, terrain::Terrain, utils::float::FloatSetExt, DEPTH_FAR,
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
    inclination: Range<f32>,
    projection: Matrix4<f32>,
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

    let incl_tan1 = if inclination.start < PI * -0.49 {
        NEG_INFINITY
    } else {
        inclination.start.tan()
    };
    let incl_tan2 = if inclination.end > PI * 0.49 {
        INFINITY
    } else {
        inclination.end.tan()
    };

    // Main loop
    mipbeamcast(
        terrain.size().truncate().cast().unwrap(),
        terrain.levels.len() as u32,
        vec2(eye.x, eye.y),
        dir1,
        dir2,
        |incidence, preproc| {
            // Localize captured variables. This does have an impact on the
            // generated assembly code.
            let output_depth = &mut output_depth[..];
            let skip_buffer = &mut skip_buffer[..];
            let (eye, projection) = (eye, projection);
            let dir_primary = dir_primary;
            let (incl_tan1, incl_tan2) = (incl_tan1, incl_tan2);

            // TODO: Early-out by Z range
            let cell = incidence.cell(preproc);

            debug_assert!((cell.mip as usize) < terrain.levels.len());
            let level = unsafe { terrain.levels.get_unchecked(cell.mip as usize) };

            let level_size_bits_x = terrain.size_bits.x - cell.mip;
            let row_index = cell.pos.x as usize + ((cell.pos.y as usize) << level_size_bits_x);
            debug_assert!(cell.pos.x < (1 << terrain.size_bits.x - cell.mip));
            debug_assert!(cell.pos.y < (1 << terrain.size_bits.y - cell.mip));
            debug_assert!(cell.pos.x >= 0);
            debug_assert!(cell.pos.y >= 0);
            debug_assert!(row_index < level.rows.len());
            let row = unsafe { level.rows.get_unchecked(row_index) };

            let dist = (cell.pos.x as f32 - eye.x) * dir_primary.x
                + (cell.pos.y as f32 - eye.y) * dir_primary.y;

            if dist <= 0.0 {
                return;
            }

            // Rasterize spans
            for span in row.iter() {
                // TODO: Calculations done here fail to be vectorized - figure
                //       out how to make it SIMD-friendly or use SIMD explicitly
                // FoV clip
                let z1 = [span.start as f32, eye.z + incl_tan1 * dist].max();
                let z2 = [span.end as f32, eye.z + incl_tan2 * dist].min();

                if z1 >= z2 {
                    continue;
                }

                // TODO: Precise calculation - we are currently naïvely projecting
                //       the centroid of each row, which might not be suitable
                //       for conservative rendering
                let p1 = projection * vec4(dist, 0.0, z1, 1.0);
                let p2 = projection * vec4(dist, 0.0, z2, 1.0);

                let (mut p1, mut p2) = (Point3::from_homogeneous(p1), Point3::from_homogeneous(p2));
                trace.opticast_span(
                    vec2(
                        (cell.pos.x as u32) << cell.mip,
                        (cell.pos.y as u32) << cell.mip,
                    ),
                    1 << cell.mip,
                    span.start as u32..span.end as u32,
                );

                p1.y *= output_depth.len() as f32;
                p2.y *= output_depth.len() as f32;

                let y1 = max(p1.y as i32 + 1, 0);
                let y2 = min(p2.y as i32, output_depth.len() as i32);
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
    fn opticast_single() {
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
        let eye = Point3::new(64.0, 64.0, 15.0);
        let mut output_depth = [0.0; 69];
        let mut skip_buffer = [0; 70];
        opticast(
            &terrain,
            azimuth,
            inclination,
            projection,
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
}
