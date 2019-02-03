//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Terrain rasterizer.
use arrayvec::ArrayVec;
use cgmath::{prelude::*, vec3, vec4, Matrix4, Point3, Rad, Vector4};
use std::{f32::consts::PI, ops::Range};

use crate::{
    debug::{NoTrace, Trace},
    utils::{
        float::FloatSetExt,
        geom::{
            inclination_intersecting_half_space, intersection_of_latitudinal_line_and_plane,
            jacobian_from_projection_matrix, spherical_to_cartesian,
            spherical_to_cartesian_d_azimuth,
        },
    },
};

/// The terrain rasterizer. This type contains a temporary storage required to
/// run the terrain rasterizer.
#[derive(Debug)]
pub struct TerrainRast {
    size: usize,
    beams: Vec<BeamInfo>,
    eye: Point3<f32>,
    samples: Vec<f32>,
}

#[derive(Debug)]
struct BeamInfo {
    azimuth: Range<f32>,
    inclination: Range<f32>,
    /// Can be zero, in which case the beam should be excluded from the process.
    num_samples: usize,
    samples_start: usize,
    /// Maps from a beam space to a beam depth buffer (`x = 0`, `y ∈ [0, 1]`)
    projection: Matrix4<f32>,
}

impl Default for BeamInfo {
    fn default() -> Self {
        Self {
            azimuth: 0.0..0.0,
            inclination: 0.0..0.0,
            num_samples: 0,
            samples_start: 0,
            projection: Matrix4::zero(),
        }
    }
}

impl TerrainRast {
    /// Construct a `TerrainRast`.
    ///
    /// `resolution` is a value used to adjust the resolution of the internal
    /// buffer. A good value to start with is the resolution (the number of
    /// pixels on each side) of the output depth image.
    pub fn new(resolution: usize) -> Self {
        Self {
            size: resolution,
            beams: Vec::with_capacity(resolution * 2),
            eye: Point3::new(0.0, 0.0, 0.0),
            samples: Vec::new(),
        }
    }

    /// Update the camera matrix (the product of projection, view, and model
    /// matrices). This triggers the recalculation of sample distribution.
    pub fn set_camera_matrix(&mut self, m: Matrix4<f32>) {
        self.set_camera_matrix_trace(m, NoTrace);
    }

    /// `set_camera_matrix` with tracing.
    pub fn set_camera_matrix_trace(&mut self, m: Matrix4<f32>, mut trace: impl Trace) {
        // Find the camera's position `[x y z]` by solving the equation
        // `M*[x y z 1] == [0 0 z' 0]` where `z'` is an arbitrary real number
        // and `M` is the camera matrix.
        //
        // Rationale: If `M` is a perspective matrix, then the projection of
        // a point `x` is [0 0 ±∞] iff `x` is at the origin of the camera.
        self.eye = Point3::from_homogeneous(m.invert().unwrap() * vec4(0.0, 0.0, 1.0, 0.0));

        // Which vanishing point is visible,
        // the zenith `[0 0 ∞]` or nadir `[0 0 -∞]`?
        // let vp_is_zenith = (m * vec4(0.0, 0.0, 1.0, 0.0)).w > 0.0;

        const VIEWPORT_VERTICES: [[f32; 2]; 4] =
            [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];

        // The viewport sizes used for azimuth range calculation and
        // inclination range calculation must differ slightly. This fixes
        // instability when a vanishing point is near the viewport border.
        // I believe the performance impact is not large since this does not
        // change the number of beam-casting operations, which has been shown
        // to be a dominant factor in my previous work.
        let safe_margin = 4.0 / self.size as f32;

        // Find the normal vectors of the viewport border edges in the model space.
        // Note that a normal vector is a bivector, thus must be multiplied with
        // the inverse transpose of a matrix.
        let m_inv = m.invert().unwrap();
        let j1 = jacobian_from_projection_matrix(
            m_inv,
            Point3::new(-(1.0 + safe_margin), -(1.0 + safe_margin), 0.5).to_homogeneous(),
        )
        .transpose()
        .invert()
        .unwrap();
        let j2 = jacobian_from_projection_matrix(
            m_inv,
            Point3::new(1.0 + safe_margin, 1.0 + safe_margin, 0.5).to_homogeneous(),
        )
        .transpose()
        .invert()
        .unwrap();
        let ms_viewport_normals = [
            j1 * vec3(0.0, -(1.0 + safe_margin), 0.0),
            j2 * vec3(1.0 + safe_margin, 0.0, 0.0),
            j2 * vec3(0.0, 1.0 + safe_margin, 0.0),
            j1 * vec3(-(1.0 + safe_margin), 0.0, 0.0),
        ];

        // They must be directed outside
        assert!(ms_viewport_normals[0].dot(ms_viewport_normals[2]) <= 0.0);
        assert!(ms_viewport_normals[1].dot(ms_viewport_normals[3]) <= 0.0);

        // Here's how to calculate the visible portion of a latitudinal line
        let inclination_range_for_azimuth = |azimuth| -> Range<f32> {
            let mut ranges = ms_viewport_normals
                .iter()
                .map(|normal| inclination_intersecting_half_space(azimuth, -*normal));

            // Take intersectons of all ranges
            let mut range: Range<f32> = ranges.next().unwrap().into();
            range = ranges.fold(range, |x, y| y & &x);
            range.start..[range.start, range.end].max()
        };

        // Calculate the range of azimuth angles visible within the viewport.
        let ms_viewport_vertex_dirs: ArrayVec<[_; 4]> = (0..4)
            .map(|i| {
                // Transform a line through `[±1, ±1, -1]` and `[±1, ±1, 1]`.
                // A half-line is obtained. Find where this half-line intersects
                // with an infinitely large sphere.
                let vert = VIEWPORT_VERTICES[i];
                let mut v = m_inv * vec4(vert[0], vert[1], 0.0, 1.0);
                let z = -v.w / m_inv.z.w; // find `z` that makes transformed `w` zero
                v += z * m_inv.z;
                debug_assert!(v.w.abs() < 0.0001);
                v.truncate()
            })
            .collect();

        let azimuth_range = {
            use std::f32::NAN;

            let mut angles: ArrayVec<[_; 4]> = ms_viewport_vertex_dirs
                .iter()
                .map(|dir| {
                    if dir.x == 0.0 && dir.y == 0.0 {
                        NAN
                    } else {
                        dir.y.atan2(dir.x)
                    }
                })
                .collect();

            // Wrap-around correction: The differences in the azimuth angles of
            // two adjacent vertices must be < 180°
            const PI2: f32 = PI * 2.0;
            for i in 1..angles.len() {
                angles[i] += ((angles[i - 1] - angles[i]) * (1.0 / PI2)).round() * PI2;
            }

            if (angles[3] - angles[0]).abs() <= PI {
                angles.min()..angles.max()
            } else {
                0.0..PI2
            }
        };

        debug_assert!(
            azimuth_range.start.is_finite()
                && azimuth_range.end.is_finite()
                && azimuth_range.start <= azimuth_range.end,
            "{:?}",
            azimuth_range
        );

        // Distribute beams in `azimuth_range`
        {
            let mut last_range = inclination_range_for_azimuth(azimuth_range.start);
            let mut last_angle = azimuth_range.start;
            self.beams.clear();

            loop {
                // Calculate the average value of `|∂Project(x)/∂φ|` on
                // the visible portion of the latitudinal line at `last_angle`
                let d1 = jacobian_from_projection_matrix(
                    m,
                    spherical_to_cartesian(last_angle, last_range.start).extend(0.0),
                ) * spherical_to_cartesian_d_azimuth(last_angle, last_range.start);
                let d2 = jacobian_from_projection_matrix(
                    m,
                    spherical_to_cartesian(last_angle, last_range.end).extend(0.0),
                ) * spherical_to_cartesian_d_azimuth(last_angle, last_range.end);
                let speed1 = d1.magnitude();
                let speed2 = d2.magnitude();

                // Adjust the interval of latitudinal lines to match the output
                // image resolution.
                let width = 2.0 / self.size as f32 / ((speed1 + speed2) * 0.5);
                debug_assert!(
                    width.is_finite(),
                    "{:?}",
                    (m, last_angle, last_range, d1, d2, width)
                );

                let end;
                let mut angle;

                angle = last_angle + width;
                if angle >= azimuth_range.end {
                    end = true;
                    angle = azimuth_range.end;
                } else {
                    end = false;
                }

                let range = inclination_range_for_azimuth(angle);

                self.beams.push(BeamInfo {
                    azimuth: last_angle..angle,
                    inclination: [range.start, last_range.start].min()
                        ..[range.end, last_range.end].max(),
                    ..BeamInfo::default()
                });

                last_angle = angle;
                last_range = range;

                if end {
                    break;
                }
            }
        }

        for beam in self.beams.iter_mut() {
            // Project the endpoints of the primary latitudinal line
            let theta = (beam.azimuth.start + beam.azimuth.end) * 0.5;
            let p1 = m * spherical_to_cartesian(theta, beam.inclination.start).extend(0.0);
            let p2 = m * spherical_to_cartesian(theta, beam.inclination.end).extend(0.0);
            let (p1, p2) = (Point3::from_homogeneous(p1), Point3::from_homogeneous(p2));

            let diff = (p2 - p1).truncate();
            let len = diff.magnitude();
            let chebyshev_len = [diff.x.abs(), diff.y.abs()].max();

            // Reject zero-length beams
            if (diff.x == 0.0 && diff.y == 0.0) || len == 0.0 || chebyshev_len == 0.0 {
                beam.num_samples = 0;
                continue;
            }

            // The preliminary sample count
            beam.num_samples = (chebyshev_len * 0.5 * self.size as f32).ceil() as usize;

            // Create a beam projection matrix
            let projection =
                // Reorient the output so that `p2 - p1` aligns to the Y axis
                Matrix4::from_cols(
                    Vector4::zero(),
                    vec4(diff.x, diff.y, 0.0, 0.0) * (1.0 / (len * len)),
                    vec4(0.0, 0.0, 1.0, 0.0),
                    vec4(0.0, 0.0, 0.0, 1.0),
                ).transpose() *
                // Move `p1` to the origin
                Matrix4::from_translation(vec3(-p1.x, -p1.y, 0.0)) *
                // The camera matrix
                m *
                // Beam space to model space
                Matrix4::from_angle_z(Rad(theta)) *
                Matrix4::from_translation(vec3(-self.eye.x, -self.eye.y, 0.0));

            beam.projection = projection;
        }

        // FIMXE: Adjust sample counts to hard-limit the total number?

        let mut samples_start = 0;
        for beam in self.beams.iter_mut() {
            // Allocate a region for the beam depth buffer
            beam.samples_start = samples_start;
            samples_start += beam.num_samples;
        }
        self.samples.resize(samples_start, 0.0);

        if trace.wants_opticast_sample() {
            for beam in self.beams.iter() {
                if beam.num_samples == 0 {
                    continue;
                }

                let theta = (beam.azimuth.start + beam.azimuth.end) * 0.5;
                let p1 = m * spherical_to_cartesian(theta, beam.inclination.start).extend(0.0);
                let p2 = m * spherical_to_cartesian(theta, beam.inclination.end).extend(0.0);
                let (p1, p2) = (Point3::from_homogeneous(p1), Point3::from_homogeneous(p2));

                let binormal = vec3(-theta.sin(), theta.cos(), 0.0);

                let unproject = |p: Point3<f32>| {
                    let mut v = m_inv * vec4(p[0], p[1], 0.0, 1.0);
                    let z = -v.w / m_inv.z.w; // find `z` that makes transformed `w` zero
                    v += z * m_inv.z;
                    v.truncate()
                };

                // Find a plane containing the viewport-space point `p` and
                // includes a line `θ = theta ± π/2, φ = 0`
                let to_plane_normal = |p: Point3<f32>| unproject(p).cross(binormal);

                let mut last_p = to_plane_normal(p1);
                for i in 1..=beam.num_samples {
                    let p = to_plane_normal(p1 + (p2 - p1) * (i as f32 / beam.num_samples as f32));

                    // What we do here is equivalent to
                    // `intersection_of_latitudinal_line_and_plane_with_tangent`,
                    // but more stable when the output of `unproject` is close to
                    // the zenith or nadir because a binormal vector is supplied
                    // explicitly.

                    let v1 = intersection_of_latitudinal_line_and_plane(beam.azimuth.start, last_p);
                    let v2 = intersection_of_latitudinal_line_and_plane(beam.azimuth.end, last_p);
                    let v3 = intersection_of_latitudinal_line_and_plane(beam.azimuth.start, p);
                    let v4 = intersection_of_latitudinal_line_and_plane(beam.azimuth.end, p);

                    trace.terrainrast_sample(&[v1, v2, v4, v3]);

                    last_p = p;
                }
            }
        }

        // end of function
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{assert_abs_diff_eq, vec3, Perspective, Point3};

    #[test]
    fn set_camera_matrix_sanity() {
        let mut rast = TerrainRast::new(64);

        let eye = dbg!(Point3::new(1.0, 2.0, -3.0));
        let p: Matrix4<f32> = Perspective {
            left: -0.5,
            right: 0.5,
            top: 0.5,
            bottom: -0.5,
            near: 1.0,
            far: 100.0,
        }
        .into();
        let v = Matrix4::look_at(eye, Point3::new(40.0, -20.0, 30.0), vec3(0.2, 0.5, 0.8));

        rast.set_camera_matrix(dbg!(p) * dbg!(v));

        let estimated_eye = dbg!(rast.eye);
        assert_abs_diff_eq!(estimated_eye, eye, epsilon = 0.001);

        dbg!(&rast.beams);
        dbg!(rast.beams.len());

        for beam in rast.beams.iter() {
            let mut p1 = spherical_to_cartesian(0.0, beam.inclination.start).extend(0.0);
            let mut p2 = spherical_to_cartesian(0.0, beam.inclination.end).extend(0.0);

            let (p1, p2) = (beam.projection * p1, beam.projection * p2);
            let (p1, p2) = (Point3::from_homogeneous(p1), Point3::from_homogeneous(p2));

            assert_abs_diff_eq!(p1, Point3::new(0.0, 0.0, p1.z), epsilon = 0.001);
            assert_abs_diff_eq!(p2, Point3::new(0.0, 1.0, p1.z), epsilon = 0.001);
        }
    }
}
