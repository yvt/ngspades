//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/// Compute the minimum value of given `f32`s. This is faster than `<f32>::min`
/// in general because it matches the semantics of x86's `minss`.
pub fn f32min(x: f32, y: f32) -> f32 {
    if x < y {
        x
    } else {
        y
    }
}

/// Compute the maximum value of given `f32`s. This is faster than `<f32>::max`
/// in general because it matches the semantics of x86's `maxss`.
pub fn f32max(x: f32, y: f32) -> f32 {
    if x > y {
        x
    } else {
        y
    }
}

pub trait FloatSetExt {
    type Float;

    /// Compute the minimum value of the set. Panics if the set is empty.
    fn min(&self) -> Self::Float;

    /// Compute the maximum value of the set. Panics if the set is empty.
    fn max(&self) -> Self::Float;
}

impl FloatSetExt for [f32] {
    type Float = f32;

    fn min(&self) -> Self::Float {
        let mut output = self[0];
        for &x in &self[1..] {
            output = f32min(output, x);
        }
        output
    }

    fn max(&self) -> Self::Float {
        let mut output = self[0];
        for &x in &self[1..] {
            output = f32max(output, x);
        }
        output
    }
}
