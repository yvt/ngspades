//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

use cgmath::prelude::*;
use cgmath::{Point2, Point3, BaseNum};

use super::{ElementWiseOp, ElementWisePartialOrd, BoolArray};

pub trait AxisAlignedBox<T> : Sized {
    type Point : EuclideanSpace + ElementWiseOp + ElementWisePartialOrd;

    fn new(min: Self::Point, max: Self::Point) -> Self;

    fn min(&self) -> Self::Point;
    fn max(&self) -> Self::Point;

    #[inline]
    fn contains_point(&self, point: &Self::Point) -> bool where T : PartialOrd {
        point.element_wise_ge(&self.min()).all() && point.element_wise_lt(&self.max()).all()
    }

    fn is_valid(&self) -> bool;
    fn is_empty(&self) -> bool;

    #[inline]
    fn size(&self) -> <Self::Point as EuclideanSpace>::Diff where T : BaseNum {
        self.max() - self.min()
    }

    #[inline]
    fn union(&self, other: &Self) -> Self where T : BaseNum {
        Self::new(self.min().element_wise_min(&other.min()),
            self.max().element_wise_max(&other.max()))
    }

    #[inline]
    fn union_assign(&mut self, other: &Self) where T : BaseNum {
        *self = self.union(other);
    }

    #[inline]
    fn intersection(&self, other: &Self) -> Option<Self> where T : BaseNum {
        let s = Self::new(self.min().element_wise_max(&other.min()),
            self.max().element_wise_min(&other.max()));
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

/// Represents an axis-aligned 2D box.
#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Box2<T> {
    /// The minimum coordinate (inclusive).
    pub min: Point2<T>,

    /// The maximum coordinate (exclusive).
    pub max: Point2<T>,
}

/// Represents an axis-aligned 3D box.
#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Box3<T> {
    /// The minimum coordinate (inclusive).
    pub min: Point3<T>,

    /// The maximum coordinate (exclusive).
    pub max: Point3<T>,
}

impl<T: BaseNum> AxisAlignedBox<T> for Box2<T> {
    type Point = Point2<T>;

    #[inline]
    fn new(min: Self::Point, max: Self::Point) -> Self {
        Self { min: min, max: max }
    }

    #[inline] fn is_valid(&self) -> bool { self.size().min() >= T::zero() }
    #[inline] fn is_empty(&self) -> bool { self.size().min() <= T::zero() }

    #[inline] fn min(&self) -> Self::Point { self.min }
    #[inline] fn max(&self) -> Self::Point { self.max }
}

impl<T: BaseNum> AxisAlignedBox<T> for Box3<T> {
    type Point = Point3<T>;

    #[inline]
    fn new(min: Self::Point, max: Self::Point) -> Self {
        Self { min: min, max: max }
    }

    #[inline] fn is_valid(&self) -> bool { self.size().min() >= T::zero() }
    #[inline] fn is_empty(&self) -> bool { self.size().min() <= T::zero() }

    #[inline] fn min(&self) -> Self::Point { self.min }
    #[inline] fn max(&self) -> Self::Point { self.max }
}

