//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{vec3, Matrix3, Matrix4, Vector3, Vector4};
use std::{f32::consts::FRAC_PI_2, ops::Range};

use crate::utils::float::FloatSetExt;

/// Calculate the Jacobian matrix of a specified projective transformation at a
/// specified point specified in homogeneous coordinates.
pub fn jacobian_from_projection_matrix(m: Matrix4<f32>, p: Vector4<f32>) -> Matrix3<f32> {
    // ((a + bt) / (c + dt)) (d/dt) |t=0 = b/c - (a/c)*d/c
    let m11 = Matrix3::from_cols(m.x.truncate(), m.y.truncate(), m.z.truncate());

    let transformed_h = m * p;
    let fac = 1.0 / transformed_h.w;
    let transformed = transformed_h.truncate() * fac;

    (m11 - Matrix3::from_cols(
        transformed * m.x.w,
        transformed * m.y.w,
        transformed * m.z.w,
    )) * fac
}

/// Find a portion on a latitudinal line where it intersects with a given
/// half-space `dot(x, normal) ≥ 0` containing the origin point, and return the
/// range of inclination angles.
pub fn inclination_intersecting_half_space(azimuth: f32, normal: Vector3<f32>) -> InclinationRange {
    // Project the plane on the one on which the latitudinal line lies
    let px = azimuth.cos();
    let py = azimuth.sin();
    let normal_2 = [normal.x * px + normal.y * py, normal.z];

    // Calculate the angle
    if normal_2[1] < 0.0 {
        InclinationRange::UpperBound(normal_2[0].atan2(-normal_2[1]))
    } else {
        InclinationRange::LowerBound((-normal_2[0]).atan2(normal_2[1]))
    }
}

#[derive(Debug, Copy, Clone)]
pub enum InclinationRange {
    /// `[-π/2, x]`
    LowerBound(f32),
    /// `[x, π/2]`
    UpperBound(f32),
}

impl InclinationRange {
    #[allow(dead_code)]
    fn lower(self) -> Option<f32> {
        match self {
            InclinationRange::LowerBound(x) => Some(x),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn upper(self) -> Option<f32> {
        match self {
            InclinationRange::UpperBound(x) => Some(x),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn endpoint(self) -> f32 {
        match self {
            InclinationRange::UpperBound(x) => x,
            InclinationRange::LowerBound(x) => x,
        }
    }
}

impl Into<Range<f32>> for InclinationRange {
    fn into(self) -> Range<f32> {
        match self {
            InclinationRange::UpperBound(x) => -FRAC_PI_2..x,
            InclinationRange::LowerBound(x) => x..FRAC_PI_2,
        }
    }
}

impl std::ops::BitAnd<&Range<f32>> for InclinationRange {
    type Output = Range<f32>;

    fn bitand(self, rhs: &Range<f32>) -> Self::Output {
        match self {
            InclinationRange::UpperBound(x) => rhs.start..[rhs.end, x].min(),
            InclinationRange::LowerBound(x) => [rhs.start, x].max()..rhs.end,
        }
    }
}

/// Convert a set of spherical coordinates `(1, azimuth, inclination)` to
/// cartesian coordinates.
pub fn spherical_to_cartesian(azimuth: f32, inclination: f32) -> Vector3<f32> {
    let (a_cos, a_sin) = (azimuth.cos(), azimuth.sin());
    let (i_cos, i_sin) = (inclination.cos(), inclination.sin());
    vec3(a_cos * i_cos, a_sin * i_cos, i_sin)
}

/// `∂spherical_to_cartesian/∂azimuth`
pub fn spherical_to_cartesian_d_azimuth(azimuth: f32, inclination: f32) -> Vector3<f32> {
    let (a_cos, a_sin) = (azimuth.cos(), azimuth.sin());
    let i_cos = inclination.cos();
    vec3(-a_sin * i_cos, a_cos * i_cos, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{assert_abs_diff_eq, prelude::*, vec3, Point3};

    #[test]
    fn jacobian_from_projection_matrix_sanity() {
        let m = Matrix4::new(
            -1.902, 0.6093, -0.920, -1.051, -0.388, 0.4639, -1.370, -1.007, 1.3520, 1.9933, -1.944,
            0.9541, 1.7110, -1.205, 1.3620, 0.7418,
        );
        for &p in &[
            Point3::new(-0.924, 1.8100, -1.763),
            Point3::new(-0.836, -0.657, 0.3840),
            Point3::new(1.9374, 1.0798, -1.575),
            Point3::new(1.0246, -0.755, 1.2199),
            Point3::new(-0.225, -0.524, 0.7021),
        ] {
            const DIF: f32 = 0.001;
            let q0 = dbg!(m.transform_point(p));
            let q1 = dbg!((m.transform_point(p + vec3(DIF, 0.0, 0.0)) - q0) / DIF);
            let q2 = dbg!((m.transform_point(p + vec3(0.0, DIF, 0.0)) - q0) / DIF);
            let q3 = dbg!((m.transform_point(p + vec3(0.0, 0.0, DIF)) - q0) / DIF);

            let j = dbg!(jacobian_from_projection_matrix(m, p.to_homogeneous()));

            assert_abs_diff_eq!(j.x, q1, epsilon = 0.001);
            assert_abs_diff_eq!(j.y, q2, epsilon = 0.001);
            assert_abs_diff_eq!(j.z, q3, epsilon = 0.001);
        }
    }

    #[test]
    fn inclination_intersecting_half_space_sanity() {
        assert_abs_diff_eq!(
            inclination_intersecting_half_space(0.0, vec3(1.0, 0.0, 0.1))
                .lower()
                .unwrap(),
            -1.47,
            epsilon = 0.1
        );
        assert_abs_diff_eq!(
            inclination_intersecting_half_space(0.0, vec3(1.0, 0.0, -0.1))
                .upper()
                .unwrap(),
            1.47,
            epsilon = 0.1
        );
        assert_abs_diff_eq!(
            inclination_intersecting_half_space(0.0, vec3(0.0, 0.0, 1.0))
                .lower()
                .unwrap(),
            0.0,
            epsilon = 0.1
        );
        assert_abs_diff_eq!(
            inclination_intersecting_half_space(0.0, vec3(0.0, 0.0, -1.0))
                .upper()
                .unwrap(),
            0.0,
            epsilon = 0.1
        );
    }
}
