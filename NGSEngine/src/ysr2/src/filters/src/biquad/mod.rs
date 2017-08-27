//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Biquad filters.

mod simple;
mod filter;
pub use self::simple::*;
pub use self::filter::*;

#[cfg(test)]
mod tests;

/// Coefficients for a normalized biquad filter with the difference equation
/// `y[n] = b0 x[n] + b1 x[n-1] + b2 x[n-2] - a1 y[n-1] - a2 y[n-2]`.
///
/// The transfer function is given by the following equation:
///
/// ```text
///       b0 + b1^(-z) + b2^(-2z)
/// Y/X = -----------------------
///        1 + a1^(-z) + a2^(-2z)
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoefs {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Default for BiquadCoefs {
    fn default() -> Self {
        Self::identity()
    }
}

impl BiquadCoefs {
    /// Construct `BiquadCoefs` representing an identity filter.
    pub fn identity() -> Self {
        BiquadCoefs {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}
