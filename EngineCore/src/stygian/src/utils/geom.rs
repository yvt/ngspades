//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{prelude::*, vec3, Matrix3, Matrix4, Vector3, Vector4};
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

/// Find the intersection of a latitudinal line and a plane. The plane is
/// defined to have `tangent` as its tangent vector and the corresponding
/// binormal vector is horizontal.
#[allow(dead_code)]
pub fn intersection_of_latitudinal_line_and_plane_with_tangent(
    azimuth: f32,
    tangent: Vector3<f32>,
) -> Vector3<f32> {
    let binormal = vec3(-tangent.y, tangent.x, 0.0);
    intersection_of_latitudinal_line_and_plane(azimuth, tangent.cross(binormal))
}

/// Find the intersection of a latitudinal line and a plane.
pub fn intersection_of_latitudinal_line_and_plane(
    azimuth: f32,
    normal: Vector3<f32>,
) -> Vector3<f32> {
    // φ=0, θ=azimuth + π/2
    let px = azimuth.cos();
    let py = azimuth.sin();
    let perp = vec3(py, -px, 0.0);

    normal.cross(perp).normalize()
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

/// Find a 3x3 matrix that unprojects a point on a viewport space to infinity.
///
/// `m` is a 4x4 matrix that transforms a 3D point from a viewport space to a
/// model space. Let `Unproject([x y])` be a function that finds `z` such that
/// `m * [x y z 1] == [x' y' z' 0]` and returns `[x' y' z']`. This function
/// returns another matrix `U` representing this operation, i.e.,
/// `U * [x y 1] = [x' y' z']`.
pub fn unprojector_xy_to_infinity(m: Matrix4<f32>) -> Matrix3<f32> {
    let t = 1.0 / m.z.w;
    let x = m.x - m.z * (m.x.w * t);
    let y = m.y - m.z * (m.y.w * t);
    let w = m.w - m.z * (m.w.w * t);
    Matrix3::from_cols(x.truncate(), y.truncate(), w.truncate())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{assert_abs_diff_eq, vec3, Point3};

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
    fn intersection_of_latitudinal_line_and_plane_with_tangent_sanity() {
        let elevation = 0.7f32;
        let yaw = 2.0f32;
        let angle = 0.7f32;

        let tangent = dbg!(spherical_to_cartesian(yaw, elevation));
        let binormal = dbg!(vec3(-tangent.y, tangent.x, 0.0).normalize());
        let p = dbg!(tangent * angle.cos() + binormal * angle.sin());

        let p_azimuth = dbg!(p.y.atan2(p.x));

        let v = intersection_of_latitudinal_line_and_plane_with_tangent(p_azimuth, tangent);

        assert_abs_diff_eq!(v.y.atan2(v.x), p_azimuth, epsilon = 0.001);
        assert_abs_diff_eq!(v, p, epsilon = 0.001);
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

    #[test]
    fn unprojector_xy_to_infinity_sanity() {
        let p_ws = vec3(0.5, 0.8, 1.3).normalize(); // point at infinity

        // FIXME: Points are unprojected to the opposite direction when this `-`
        //        is removed. Investigate why
        let m = -Matrix4::new(
            -1.902, 0.6093, -0.920, -1.051, -0.388, 0.4639, -1.370, -1.007, 1.3520, 1.9933, -1.944,
            0.9541, 1.7110, -1.205, 1.3620, 0.7418,
        );
        let m_inv = dbg!(m.invert().unwrap());
        let m_unproj = dbg!(unprojector_xy_to_infinity(m_inv));

        let p = dbg!(m * p_ws.extend(0.0));
        let p = Point3::from_homogeneous(p);

        let p_ws2 = dbg!(m_unproj * vec3(p.x, p.y, 1.0));
        assert_abs_diff_eq!(p_ws2.normalize(), p_ws, epsilon = 0.001);
    }
}
