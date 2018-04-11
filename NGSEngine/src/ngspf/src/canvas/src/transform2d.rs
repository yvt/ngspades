//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{num_traits::NumCast, prelude::*, BaseFloat, Matrix3, Point2, Rad, Vector2};
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

/// 2 x 3 matrix representing a 2D affine transformation.
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Affine2<S>(Matrix3<S>);

impl<S: BaseFloat> Affine2<S> {
    /// Create a new matrix, providing values for each element.
    #[inline]
    pub fn new(c0r0: S, c0r1: S, c1r0: S, c1r1: S, c2r0: S, c2r1: S) -> Affine2<S> {
        Affine2(Matrix3::new(
            c0r0,
            c0r1,
            S::zero(),
            c1r0,
            c1r1,
            S::zero(),
            c2r0,
            c2r1,
            S::one(),
        ))
    }

    /// Construct a `Affine2` from a given `Matrix3` by assuming the last row is
    /// `(0 0 1)`.
    #[inline]
    pub fn from_matrix3_truncate(m: Matrix3<S>) -> Affine2<S> {
        Self::new(m.x.x, m.x.y, m.y.x, m.y.y, m.z.x, m.z.y)
    }

    /// Get a 2D transformation matrix representing the same transformation.
    #[inline]
    pub fn as_matrix3(&self) -> Matrix3<S> {
        self.0
    }

    /// Create a translation matrix from a vector.
    pub fn from_translation(x: Vector2<S>) -> Self {
        let one = One::one();
        let zero = Zero::zero();
        Self::new(one, zero, zero, one, x.x, x.y)
    }

    /// Create a rotation matrix.
    pub fn from_angle<A: Into<Rad<S>>>(theta: A) -> Self {
        Self::from_matrix3_truncate(Matrix3::from_angle_z(theta))
    }

    /// Create a scaling matrix from a set of scale values.
    pub fn from_nonuniform_scale(x: S, y: S) -> Self {
        let zero = Zero::zero();
        Self::new(x, zero, zero, y, zero, zero)
    }

    /// Create a scaling matrix from a scale value.
    pub fn from_scale(x: S) -> Self {
        let zero = Zero::zero();
        Self::new(x, zero, zero, x, zero, zero)
    }
}

impl<S: NumCast + Copy> Affine2<S> {
    /// Cast each element to another type.
    pub fn cast<T: NumCast>(&self) -> Affine2<T> {
        Affine2(self.0.cast::<T>())
    }
}

impl<S: BaseFloat> Into<Matrix3<S>> for Affine2<S> {
    fn into(self) -> Matrix3<S> {
        self.0
    }
}

impl<S: BaseFloat> Zero for Affine2<S> {
    #[inline]
    fn zero() -> Self {
        Self::from_matrix3_truncate(Zero::zero())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0.x.x.is_zero() && self.0.x.y.is_zero() && self.0.y.x.is_zero() && self.0.y.y.is_zero()
            && self.0.z.x.is_zero() && self.0.z.y.is_zero()
    }
}

impl<S: BaseFloat> One for Affine2<S> {
    #[inline]
    fn one() -> Self {
        Affine2(One::one())
    }
}

impl<S: BaseFloat> Add for Affine2<S> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::from_matrix3_truncate(self.0 + rhs.0)
    }
}

impl<'a, S: BaseFloat> Add<&'a Affine2<S>> for Affine2<S> {
    type Output = Self;
    fn add(self, rhs: &'a Self) -> Self {
        self + *rhs
    }
}

impl<S: BaseFloat> Sub for Affine2<S> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::from_matrix3_truncate(self.0 - rhs.0)
    }
}

impl<'a, S: BaseFloat> Sub<&'a Affine2<S>> for Affine2<S> {
    type Output = Self;
    fn sub(self, rhs: &'a Self) -> Self {
        self - *rhs
    }
}

impl<S: BaseFloat> Mul for Affine2<S> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::from_matrix3_truncate(self.0 * rhs.0)
    }
}

impl<'a, S: BaseFloat> Mul<&'a Affine2<S>> for Affine2<S> {
    type Output = Self;
    fn mul(self, rhs: &'a Self) -> Self {
        self * *rhs
    }
}

impl<S: BaseFloat> AddAssign for Affine2<S> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<S: BaseFloat> SubAssign for Affine2<S> {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<S: BaseFloat> MulAssign for Affine2<S> {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<S: BaseFloat> Transform<Point2<S>> for Affine2<S> {
    fn one() -> Self {
        Affine2(Transform::<Point2<S>>::one())
    }

    fn look_at(eye: Point2<S>, center: Point2<S>, up: Vector2<S>) -> Affine2<S> {
        Affine2::from_matrix3_truncate(Transform::look_at(eye, center, up))
    }

    fn transform_vector(&self, vec: Vector2<S>) -> Vector2<S> {
        Transform::<Point2<S>>::transform_vector(&self.0, vec)
    }

    fn transform_point(&self, point: Point2<S>) -> Point2<S> {
        Transform::<Point2<S>>::transform_point(&self.0, point)
    }

    fn concat(&self, other: &Affine2<S>) -> Affine2<S> {
        *self * other
    }

    fn inverse_transform(&self) -> Option<Affine2<S>> {
        Transform::<Point2<S>>::inverse_transform(&self.0).map(Affine2::from_matrix3_truncate)
    }
}
