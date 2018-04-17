//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides bi-quad filter designs for audio equalization.
//!
//! The derivations are based on the well-known document named
//! "[Cookbook formulae for audio EQ biquad filter coefficients]" by Robert
//! Bristow-Johnson.
//!
//! Frequency values are normalized and must be specified in the range `[0, 0.5]`.
//!
//! [Cookbook formulae for audio EQ biquad filter coefficients]: http://www.musicdsp.org/files/Audio-EQ-Cookbook.txt
use biquad::BiquadCoefs;
use std::f64::consts::PI;

/// Construct a `BiquadCoefs` for a low-pass filter with a given cutoff
/// frequency `f0` and Q value `q`.
///
/// This filter is derived from the following analog prototype in the s-domain
/// (for normalized frequency):
///
/// ```text
///              1
/// H(s) = ---------------
///         s^2 + s/q + 1
/// ```
pub fn low_pass_filter(f0: f64, q: f64) -> BiquadCoefs {
    debug_assert!(f0 >= 0.0 && f0 <= 0.5);
    debug_assert!(q > 0.0);
    let w0 = f0 * (PI * 2.0);
    let cos = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let b0 = (1.0 - cos) * 0.5;
    let b1 = 1.0 - cos;
    let b2 = (1.0 - cos) * 0.5;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos;
    let a2 = 1.0 - alpha;
    BiquadCoefs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// Construct a `BiquadCoefs` for a high-pass filter with a given cutoff
/// frequency `f0` and Q value `q`.
///
/// This filter is derived from the following analog prototype in the s-domain
/// (for normalized frequency):
///
/// ```text
///              s^2
/// H(s) = ---------------
///         s^2 + s/q + 1
/// ```
pub fn high_pass_filter(f0: f64, q: f64) -> BiquadCoefs {
    debug_assert!(f0 >= 0.0 && f0 <= 0.5);
    debug_assert!(q > 0.0);
    let w0 = f0 * (PI * 2.0);
    let cos = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let b0 = (1.0 + cos) * 0.5;
    let b1 = -1.0 - cos;
    let b2 = (1.0 + cos) * 0.5;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos;
    let a2 = 1.0 - alpha;
    BiquadCoefs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// Construct a `BiquadCoefs` for a band-pass filter with a given center
/// frequency `f0` and Q value `q`.
///
/// This filter is derived from the following analog prototype in the s-domain
/// (for normalized frequency):
///
/// ```text
///              s/q
/// H(s) = ---------------
///         s^2 + s/q + 1
/// ```
pub fn band_pass_filter(f0: f64, q: f64) -> BiquadCoefs {
    debug_assert!(f0 >= 0.0 && f0 <= 0.5);
    debug_assert!(q > 0.0);
    let w0 = f0 * (PI * 2.0);
    let alpha = w0.sin() / (2.0 * q);
    let b0 = alpha;
    let b1 = 0.0;
    let b2 = -alpha;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * w0.cos();
    let a2 = 1.0 - alpha;
    BiquadCoefs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// Construct a `BiquadCoefs` for a notch filter with a given center frequency
/// `f0` and Q value `q`.
///
/// This filter is derived from the following analog prototype in the s-domain
/// (for normalized frequency):
///
/// ```text
///            s^2 + 1
/// H(s) = ---------------
///         s^2 + s/q + 1
/// ```
pub fn notch_filter(f0: f64, q: f64) -> BiquadCoefs {
    debug_assert!(f0 >= 0.0 && f0 <= 0.5);
    debug_assert!(q > 0.0);
    let w0 = f0 * (PI * 2.0);
    let cos = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let b0 = 1.0;
    let b1 = cos * -2.0;
    let b2 = 1.0;
    let a0 = 1.0 + alpha;
    let a1 = cos * -2.0;
    let a2 = 1.0 - alpha;
    BiquadCoefs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// Construct a `BiquadCoefs` for an all-pass filter with a given midpoint
/// frequency `f0` and Q value `q`.
///
/// This filter is derived from the following analog prototype in the s-domain
/// (for normalized frequency):
///
/// ```text
///         s^2 - s/q + 1
/// H(s) = ---------------
///         s^2 + s/q + 1
/// ```
pub fn all_pass_filter(f0: f64, q: f64) -> BiquadCoefs {
    debug_assert!(f0 >= 0.0 && f0 <= 0.5);
    debug_assert!(q > 0.0);
    let w0 = f0 * (PI * 2.0);
    let cos = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let b0 = 1.0 - alpha;
    let b1 = cos * -2.0;
    let b2 = 1.0 + alpha;
    let a0 = 1.0 + alpha;
    let a1 = cos * -2.0;
    let a2 = 1.0 - alpha;
    BiquadCoefs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// Construct a `BiquadCoefs` for a peaking equalization filter with a given
/// center frequency `f0`, Q value `q`, and gain `a`.
///
/// This filter is derived from the following analog prototype in the s-domain
/// (for normalized frequency):
///
/// ```text
///         s^2 + s*(a/q) + 1
/// H(s) = -------------------
///         s^2 + s/(a/q) + 1
/// ```
pub fn peaking_eq_filter(f0: f64, q: f64, a: f64) -> BiquadCoefs {
    debug_assert!(f0 >= 0.0 && f0 <= 0.5);
    debug_assert!(q > 0.0);
    debug_assert!(a > 0.0);
    let w0 = f0 * (PI * 2.0);
    let cos = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let b0 = 1.0 + alpha * a;
    let b1 = cos * -2.0;
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = cos * -2.0;
    let a2 = 1.0 - alpha / a;
    BiquadCoefs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// Construct a `BiquadCoefs` for a low shelf filter with a given corner
/// frequency `f0`, Q value `q`, and gain `a`.
///
/// This filter is derived from the following analog prototype in the s-domain
/// (for normalized frequency):
///
/// ```text
///         a * (s^2 + s*(sqrt(a)/q) + a)
/// H(s) = -------------------------------
///           s^2*a + s*(sqrt(a)/q) + 1
/// ```
pub fn low_shelf_filter(f0: f64, q: f64, a: f64) -> BiquadCoefs {
    debug_assert!(f0 >= 0.0 && f0 <= 0.5);
    debug_assert!(q > 0.0);
    debug_assert!(a >= 0.0);
    let w0 = f0 * (PI * 2.0);
    let cos = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let t = 2.0 * a.sqrt() * alpha;
    let b0 = a * ((a + 1.0) - (a - 1.0) * cos + t);
    let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos);
    let b2 = a * ((a + 1.0) - (a - 1.0) * cos - t);
    let a0 = (a + 1.0) + (a - 1.0) * cos + t;
    let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos);
    let a2 = (a + 1.0) + (a - 1.0) * cos - t;
    BiquadCoefs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// Construct a `BiquadCoefs` for a high shelf filter with a given corner
/// frequency `f0`, Q value `q`, and gain `a`.
///
/// This filter is derived from the following analog prototype in the s-domain
/// (for normalized frequency):
///
/// ```text
///         a * (s^2*a + s*(sqrt(a)/q) + 1)
/// H(s) = ---------------------------------
///             s^2 + s*(sqrt(a)/q) + a
/// ```
pub fn high_shelf_filter(f0: f64, q: f64, a: f64) -> BiquadCoefs {
    debug_assert!(f0 >= 0.0 && f0 <= 0.5);
    debug_assert!(q > 0.0);
    debug_assert!(a >= 0.0);
    let w0 = f0 * (PI * 2.0);
    let cos = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let t = 2.0 * a.sqrt() * alpha;
    let b0 = a * ((a + 1.0) + (a - 1.0) * cos + t);
    let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos);
    let b2 = a * ((a + 1.0) + (a - 1.0) * cos - t);
    let a0 = (a + 1.0) - (a - 1.0) * cos + t;
    let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos);
    let a2 = (a + 1.0) - (a - 1.0) * cos - t;
    BiquadCoefs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}
