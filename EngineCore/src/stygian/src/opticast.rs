//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use alt_fp::{fma, u16_to_f32, u23_to_f32, FloatOrdSet, SimdExt};
use cgmath::{prelude::*, vec2, vec4, Matrix4, Point3, Vector4};
use lazy_static::lazy_static;
use packed_simd::{f32x4, i32x4, shuffle};
use prefetch::prefetch as pf;
use std::{mem::uninitialized, ops::Range};

use crate::{
    cov::{CovBuffer, CovPainter},
    mipbeamcast::{mipbeamcast, F_FAC_F},
    terrain::{Span, Terrain},
    DEPTH_FAR,
};

#[derive(Debug)]
struct OpticastIncidence {
    /// This is actually a reference whose lifetime is tied to the body of
    /// `opticast`. However, the caller cannot pre-allocate
    /// `Vec<OpticastIncidence>` if this is defined as a reference.
    row: *const Vec<Span>,
    /// Ditto.
    row_slice: *const [Span],
    distance_range: [f32; 2],
}

/// The number of iterations between prefetching a row pointer and row contents.
const ROW_PREFETCH_DELAY: usize = 16;

#[derive(Debug)]
pub struct OpticastIncidenceBuffer(Vec<OpticastIncidence>);

impl OpticastIncidenceBuffer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::with_capacity(ROW_PREFETCH_DELAY)
    }

    pub fn with_capacity(size: usize) -> Self {
        Self(Vec::with_capacity(size + ROW_PREFETCH_DELAY))
    }
}

/// Perfom a beam casting and create a 1D depth image.
///
/// `cov_buffer` is a coverage buffer that must have `output_depth.len()`
/// elements.
#[inline(never)]
pub fn opticast(
    terrain: &Terrain,
    azimuth: Range<f32>,
    _inclination: Range<f32>,
    mut projection: Matrix4<f32>,
    lateral_projection: Matrix4<f32>,
    eye: Point3<f32>,
    output_depth: &mut [f32],
    cov_buffer: &mut impl CovBuffer,
    incidence_buffer: &mut OpticastIncidenceBuffer,
) {
    if output_depth.len() == 0 {
        return;
    }

    // Prepare the coverage buffer
    assert!(
        output_depth.len() <= 0x40000000,
        "beam depth buffer is too large"
    );
    cov_buffer.resize(output_depth.len() as u32);

    // Prepare beam-casting
    let dir1 = vec2(azimuth.start.cos(), azimuth.start.sin());
    let dir2 = vec2(azimuth.end.cos(), azimuth.end.sin());
    let theta = (azimuth.start + azimuth.end) * 0.5;
    let dir_primary = vec2(theta.cos(), theta.sin());

    // Set up frustum termination. A beam is terminated if `dist * fac < ref`
    // becomes false for any components.
    let (terminate_factor, terminate_ref) = {
        let (fac0, fac1, fac2);
        let (ref0, ref1, ref2);
        let depth = terrain.size().z as f32;

        let start1 = projection * vec4(0.0, 0.0, 0.0, 1.0);
        let start2 = projection * vec4(0.0, 0.0, depth, 1.0);
        let dir = projection.x;

        // y_beam >= 0
        if dir.y >= 0.0 {
            fac0 = 0.0;
            ref0 = 1.0;
        } else {
            fac0 = -dir.y;
            ref0 = [start1.y, start2.y].fmax();
        };

        // y_beam <= 1 (y_cs - w_cs <= 0)
        if dir.y - dir.w <= 0.0 {
            fac1 = 0.0;
            ref1 = 1.0;
        } else {
            fac1 = dir.y - dir.w;
            ref1 = [start1.w - start1.y, start2.w - start2.y].fmax();
        };

        // z_beam >= 0
        if dir.z >= 0.0 {
            // Actually, reaching here means something went wrong, though...
            fac2 = 0.0;
            ref2 = 1.0;
        } else {
            fac2 = -dir.z;
            ref2 = [start1.z, start2.z].fmax();
        };

        (
            f32x4::new(fac0, fac1, fac2, 0.0),
            f32x4::new(ref0, ref1, ref2, 1.0),
        )
    };

    // Scale the beam projection matrix
    projection.x.y *= output_depth.len() as f32;
    projection.y.y *= output_depth.len() as f32;
    projection.z.y *= output_depth.len() as f32;
    projection.w.y *= output_depth.len() as f32;

    let incidence_buffer = &mut incidence_buffer.0;
    incidence_buffer.truncate(ROW_PREFETCH_DELAY);

    lazy_static! {
        static ref INVALID_ROW: Vec<Span> = vec![0..0];
    }
    let invalid_row = &*INVALID_ROW;
    pf::prefetch::<pf::Read, pf::High, pf::Data, _>(invalid_row);

    if incidence_buffer.len() < ROW_PREFETCH_DELAY {
        for _ in 0..ROW_PREFETCH_DELAY {
            incidence_buffer.push(OpticastIncidence {
                row: invalid_row,
                row_slice: invalid_row.as_slice(),
                distance_range: Default::default(),
            })
        }
    }

    // Main loop
    let mut includes_start = false;
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
            let mut terrain_size = terrain_size;
            if preproc.swap_xy() {
                std::mem::swap(&mut local_dir_primary.x, &mut local_dir_primary.y);
                std::mem::swap(&mut local_eye.x, &mut local_eye.y);
                std::mem::swap(&mut terrain_size.x, &mut terrain_size.y);
            }
            if preproc.flip_x() {
                local_dir_primary.x = -local_dir_primary.x;
                local_eye.x = terrain_size.x as f32 - local_eye.x;
            }
            if preproc.flip_y() {
                local_dir_primary.y = -local_dir_primary.y;
                local_eye.y = terrain_size.y as f32 - local_eye.y;
            }

            let local_eye_dist = vec2(local_eye.x, local_eye.y).dot(local_dir_primary);

            (preproc, local_dir_primary, local_eye_dist)
        },
        |incidence, &mut (preproc, local_dir_primary, local_eye_dist)| {
            // Get the row
            let cell = incidence.cell(&preproc);

            debug_assert!((cell.mip as usize + 1) < terrain.levels.len());
            let level = unsafe { terrain.levels.get_unchecked(cell.mip as usize + 1) };

            let level_size_bits_x = terrain.size_bits.x - cell.mip;
            let row_index = cell.pos.x as usize + ((cell.pos.y as usize) << level_size_bits_x);
            debug_assert!(cell.pos.x < (1 << terrain.size_bits.x - cell.mip) - 1);
            debug_assert!(cell.pos.y < (1 << terrain.size_bits.y - cell.mip) - 1);
            debug_assert!(cell.pos.x >= 0);
            debug_assert!(cell.pos.y >= 0);
            debug_assert!(row_index < level.rows.len());
            let row = unsafe { level.rows.get_unchecked(row_index) };

            // Find the left/right-most intersections
            use packed_simd::Cast;
            let cell_raw_pos = incidence.cell_raw.pos_min();
            let cell_raw_pos = i32x4::new(cell_raw_pos.x, cell_raw_pos.y, 0, 0);
            let cell_raw_pos_f: f32x4 = cell_raw_pos.cast();
            let cell_size_f = incidence.cell_raw.size_f();

            let local_dir_primary_s =
                f32x4::new(local_dir_primary.x, local_dir_primary.y, 0.0, 0.0);

            let intersections_x = i32x4::new(
                incidence.intersections_raw[0][0].x, // beam1, enter
                incidence.intersections_raw[1][0].x, // beam1, leave
                incidence.intersections_raw[0][1].x, // beam2, enter
                incidence.intersections_raw[1][1].x, // beam2, leave
            );
            let intersections_y = i32x4::new(
                incidence.intersections_raw[0][0].y,
                incidence.intersections_raw[1][0].y,
                incidence.intersections_raw[0][1].y,
                incidence.intersections_raw[1][1].y,
            );
            let intersections_x = intersections_x.cast(): f32x4 * (1.0 / F_FAC_F);
            let mut intersections_y = intersections_y.cast(): f32x4 * (1.0 / F_FAC_F);

            if preproc.slope2_neg() {
                intersections_y = shuffle!(
                    intersections_y,
                    f32x4::splat(cell_size_f) - intersections_y,
                    [0, 1, 6, 7]
                );
            }

            // `dot(cell_raw_pos_f, dir_primary) - local_eye_dist`
            let cell_dist = local_dir_primary_s.dot2_splat(cell_raw_pos_f) - local_eye_dist;

            // `dot(x, dir_primary)` for each intersections of the beam and the cell
            //
            //     dot(x, dir_primary) - local_eye_dist
            //      = dot(cell_raw_pos_f
            //            + [cell_size_f - intersection_x, cell_size_f - intersection_y],
            //          dir_primary) - local_eye_dist
            //      = dot(cell_raw_pos_f, dir_primary) - local_eye_dist
            //        + dot([cell_size_f, cell_size_f], dir_primary)
            //        - dot([intersection_x, intersection_y], dir_primary)
            //      = cell_dist
            //        + dot([cell_size_f, cell_size_f], dir_primary)
            //        - dot([intersection_x, intersection_y], dir_primary)
            //
            let intersction_dists = cell_dist
                + f32x4::splat(cell_size_f).dot2_splat(local_dir_primary_s)
                - fma![
                    intersections_x * (f32x4::splat(local_dir_primary.x))
                        + (intersections_y * f32x4::splat(local_dir_primary.y))
                ];

            // let intersction_dists = [intersction_dists[0].fmax(), intersction_dists[1].fmin()];
            let d1 = intersction_dists;
            let d2 = shuffle!(d1, [2, 3, 0, 1]);
            let intersction_dists = [[d1, d2].fmax().extract(0), [d1, d2].fmin().extract(1)];

            // Check termination
            if (terminate_factor * intersction_dists[0])
                .ge(terminate_ref)
                .any()
            {
                return true;
            }

            let incidence_buffer = &mut *incidence_buffer;
            incidence_buffer.push(OpticastIncidence {
                row,
                row_slice: unsafe { uninitialized() },
                distance_range: intersction_dists,
            });

            // Prefetch the row
            pf::prefetch::<pf::Read, pf::High, pf::Data, _>(row);

            // Prefetch the row contents from a previous iteration
            {
                let len = incidence_buffer.len();
                let prev_incidence =
                    unsafe { incidence_buffer.get_unchecked_mut(len - ROW_PREFETCH_DELAY) };
                let slice = unsafe { &*prev_incidence.row }.as_slice();
                prev_incidence.row_slice = slice;

                pf::prefetch::<pf::Read, pf::High, pf::Data, _>(slice.as_ptr());
            }

            includes_start |= incidence.includes_start;

            // Do not terminate the beam casting yet...
            false
        },
    );

    // Process pending prefetches
    for i in 0..ROW_PREFETCH_DELAY {
        let len = incidence_buffer.len();
        let prev_incidence =
            unsafe { incidence_buffer.get_unchecked_mut(len - ROW_PREFETCH_DELAY + i) };
        let slice = unsafe { &*prev_incidence.row }.as_slice();
        prev_incidence.row_slice = slice;

        pf::prefetch::<pf::Read, pf::High, pf::Data, _>(slice.as_ptr());
    }

    // Process incidences
    let mut it = incidence_buffer[ROW_PREFETCH_DELAY..].iter();
    if includes_start {
        let first_incidence = it.next().unwrap();
        let row = unsafe { &*first_incidence.row };

        // The camera is inside this row. Draw the floor/ceiling instead.
        let floor_ceil = floor_and_ceiling_of_row(eye.z, row);

        for (i, &z) in floor_ceil.iter().enumerate() {
            if z == NO_FLOOR_CEILING {
                continue;
            }

            let z = u23_to_f32(z);
            // let span_near_dist = 0.0; // just below/above of the camera!
            let span_far_dist = first_incidence.distance_range[1];

            let mut p1 = projection.w + projection.z * z /*+ projection.x * span_near_dist*/;
            let mut p2 = projection.w + projection.z * z + projection.x * span_far_dist;

            // Apply the lateral projection matrix.
            // The left and right edges have different Z values. The matrix
            // compensates for that.
            let p1_lat = /*lateral_projection.x * span_near_dist +*/ lateral_projection.z * z;
            let p2_lat = lateral_projection.x * span_far_dist + lateral_projection.z * z;
            // D[(a + ct) / (b + dt), t = 0] = (bc - ad) / b²
            // Use this approximation to find the minimum Z value for each
            // of the top and bottom edges.
            p1.z -= (p1.z * p1_lat.w - p1.w * p1_lat.z).abs() / p1.w;
            p2.z -= (p2.z * p2_lat.w - p2.w * p2_lat.z).abs() / p2.w;

            // Clip the line segment by the plane `z == w` (near plane)
            let (p1, p2) = if let Some((p1, p2)) = clip_near_plane(p1, p2) {
                (p1, p2)
            } else {
                // Completely clipped
                continue;
            };

            // Rasterize the span
            let (mut p1, mut p2) = (Point3::from_homogeneous(p1), Point3::from_homogeneous(p2));

            // `p1.y` should be already close enough to one of the ends, but
            // snap the value so that no gaps can be seen
            p1.y = [0.0, output_depth.len() as f32][i];

            if p1.y > p2.y {
                std::mem::swap(&mut p1.y, &mut p2.y);
                std::mem::swap(&mut p1.z, &mut p2.z);
            }

            unsafe {
                paint_span(p1, p2, &mut output_depth[..], &mut *cov_buffer);
            }
        }
    }

    // Process the rest of the incidences
    while let Some(incidence) = it.next() {
        let row = unsafe { &*incidence.row_slice };

        // Rasterize spans
        let mut spans = row.iter();
        while let Some(span) = spans.next() {
            let z1 = u16_to_f32(span.start);
            let z2 = u16_to_f32(span.end);

            // Find the “reverse” AABB (like the incircle of a triangle)
            let bottom_above_eye = z1 > eye.z;
            let top_below_eye = z2 < eye.z;
            let span_bottom_dist = incidence.distance_range[bottom_above_eye as usize];
            let span_top_dist = incidence.distance_range[top_below_eye as usize];

            let mut p1 = projection.w + projection.x * span_bottom_dist + projection.z * z1;
            let mut p2 = projection.w + projection.x * span_top_dist + projection.z * z2;

            // Apply the lateral projection matrix.
            // The left and right edges have different Z values. The matrix
            // compensates for that.
            let p1_lat = lateral_projection.x * span_bottom_dist + lateral_projection.z * z1;
            let p2_lat = lateral_projection.x * span_top_dist + lateral_projection.z * z2;
            // D[(a + ct) / (b + dt), t = 0] = (bc - ad) / b²
            // Use this approximation to find the minimum Z value for each
            // of the top and bottom edges.
            p1.z -= (p1.z * p1_lat.w - p1.w * p1_lat.z).abs() / p1.w;
            p2.z -= (p2.z * p2_lat.w - p2.w * p2_lat.z).abs() / p2.w;

            // Clip the line segment by the plane `z == w` (near plane)
            let (p1, p2) = if let Some((p1, p2)) = clip_near_plane(p1, p2) {
                (p1, p2)
            } else {
                // Completely clipped
                continue;
            };

            // Rasterize the span
            let (p1, p2) = (Point3::from_homogeneous(p1), Point3::from_homogeneous(p2));

            unsafe {
                paint_span(p1, p2, &mut output_depth[..], &mut *cov_buffer);
            }
        }
    }

    // Draw sky
    cov_buffer.paint_all(SkyPainter {
        output_depth: &mut output_depth[..],
    });
}

const NO_FLOOR_CEILING: u32 = 0xffffffff;

/// Get the Z coordinates of the floor and ceiling (if any) assuming the
/// camera is inside a given row. Returns `NO_FLOOR_CEILING` if no spans were
/// found for each direction.
fn floor_and_ceiling_of_row(eye: f32, row: &[Range<u16>]) -> [u32; 2] {
    let eye = eye as i32;

    let mut last = NO_FLOOR_CEILING;
    for span in row.iter() {
        if span.start as i32 >= eye {
            return [last, span.start as u32];
        }
        last = span.end as u32;
    }

    [last, NO_FLOOR_CEILING]
}

/// Clip the line segment by the plane `z == w` (near plane)
#[inline]
fn clip_near_plane(p1: Vector4<f32>, p2: Vector4<f32>) -> Option<(Vector4<f32>, Vector4<f32>)> {
    let clip_states = [p1.z > p1.w, p2.z > p2.w];
    let clip_state = clip_states[0] as usize + clip_states[1] as usize;
    if clip_state == 2 {
        // Completely clipped
        None
    } else if clip_state == 1 {
        // Partial clipped
        let dot1 = p1.z - p1.w;
        let dot2 = p2.z - p2.w;
        let fraction = dot1 / (dot1 - dot2);
        debug_assert!(fraction >= 0.0 && fraction <= 1.0);
        let mut midpoint = p1.lerp(p2, fraction);
        midpoint.w = midpoint.z;
        if clip_states[0] {
            Some((midpoint, p2))
        } else {
            Some((p1, midpoint))
        }
    } else {
        Some((p1, p2))
    }
}

/// Unsafety: `cov_buffer` must have been `resize`d with `output_depth.len()`.
#[inline(always)]
unsafe fn paint_span(
    p1: Point3<f32>,
    p2: Point3<f32>,
    output_depth: &mut [f32],
    cov_buffer: &mut impl CovBuffer,
) {
    let y1 = [p1.y.ceil(), 0.0].fmax() as i32;
    let y2 = [p2.y, output_depth.len() as f32].fmin() as i32;
    if y1 >= y2 {
        return;
    }

    let (y1, y2) = (y1 as u32, y2 as u32);
    let delta_z = (p2.z - p1.z) / (p2.y - p1.y);
    let mut start_z = p1.z + delta_z * (u23_to_f32(y1) - p1.y);

    // The minimum value of the function `y = ax + b` within the interval `[n, n + 1]`
    // is `a(n + 1)` (if `a < 0`) or `an` (otherwise).
    if delta_z < 0.0 {
        start_z += delta_z;
    }

    cov_buffer.paint(
        y1..y2,
        SpanPainter {
            output_depth,
            base_z: start_z - (y1 as f32 * delta_z),
            delta_z,
        },
    );
}

struct SpanPainter<'a> {
    output_depth: &'a mut [f32],
    base_z: f32,
    delta_z: f32,
}
impl CovPainter for SpanPainter<'_> {
    #[inline]
    fn paint(&mut self, i: u32) {
        let output_depth = &mut self.output_depth[..];

        *unsafe { output_depth.get_unchecked_mut(i as usize) } =
            fma![(self.delta_z) * (i as f32) + (self.base_z)];
    }
}

struct SkyPainter<'a> {
    output_depth: &'a mut [f32],
}
impl CovPainter for SkyPainter<'_> {
    #[inline]
    fn paint(&mut self, i: u32) {
        *unsafe { self.output_depth.get_unchecked_mut(i as usize) } = DEPTH_FAR;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::cov::SkipBuffer;

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
        let mut cov_buffer = SkipBuffer::default();
        cov_buffer.reserve(69);
        opticast(
            &terrain,
            azimuth,
            inclination,
            projection,
            lateral_projection,
            eye,
            &mut output_depth,
            &mut cov_buffer,
            &mut OpticastIncidenceBuffer::new(),
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
        let mut cov_buffer = SkipBuffer::default();
        cov_buffer.reserve(69);
        opticast(
            &terrain,
            azimuth,
            inclination,
            projection,
            lateral_projection,
            eye,
            &mut output_depth,
            &mut cov_buffer,
            &mut OpticastIncidenceBuffer::new(),
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
