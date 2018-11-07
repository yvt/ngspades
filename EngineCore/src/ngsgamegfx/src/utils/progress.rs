//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/// Indicates the progression of an operation.
///
/// This type is comprised of two `u64` fields: `current` and `max`.
/// `current` is a numeric value in an arbitrary unit that monotonically
/// increases from `0` to `max`, at which point the corresponding operation is
/// considered complete.
#[derive(Debug, Default, Copy, Clone)]
pub struct Progress {
    current: u64,
    max: u64,
}

impl Progress {
    /// Construct a `Progress` value.
    ///
    /// # Panics
    ///
    /// This function panics if `current` is greater than `max`.
    ///
    pub fn new(current: u64, max: u64) -> Self {
        assert!(max >= current, "max: {}, current: {}", max, current);
        Self { current, max }
    }

    /// A value indicating the progression of an operation. The unit is not
    /// specified.
    ///
    /// This value increases monotonically during an operation.
    /// The upper bound of `current()` is `max()`, which represents
    /// the completion of an operation.
    pub fn current(&self) -> u64 {
        self.current
    }

    /// The maximum value of `current()`.
    ///
    /// An operation is considered complete when `current()` reaches `max()`.
    pub fn max(&self) -> u64 {
        self.max
    }

    /// Return `true` if the operation is complete.
    pub fn is_completed(&self) -> bool {
        self.current >= self.max
    }

    /// Convert a given `Progress` to a percentage (a real number in
    /// range `[0, 1]`).
    ///
    /// Returns a NaN value if `max` is `0`.
    pub fn percentage(&self) -> f64 {
        self.current as f64 / self.max as f64
    }
}

impl From<bool> for Progress {
    fn from(x: bool) -> Self {
        if x {
            Progress::new(1, 1)
        } else {
            Progress::new(0, 1)
        }
    }
}

/// Implements element-wise addition.
impl std::ops::Add for Progress {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Progress {
            current: self.current + rhs.current,
            max: self.max + rhs.max,
        }
    }
}

impl std::iter::Sum for Progress {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Progress::default(), |x, y| x + y)
    }
}

impl<'a> std::iter::Sum<&'a Progress> for Progress {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.cloned().sum()
    }
}
