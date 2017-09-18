//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Common utilities. Not intended for a public use.
#![doc(hidden)]

/// Perform convolution given two data serieses in the half complex format in
/// the frequency domain.
pub fn spectrum_convolve_inplace(x: &mut [f32], y: &[f32]) {
    assert_eq!(x.len(), y.len());

    // A (cyclic) convolution in the time domain can be accomplished by the
    // pointwise product in the frequency domain.
    x[0] *= y[0];
    x[1] *= y[1];

    unsafe {
        for i in 1..x.len() / 2 {
            let (r1, i1) = (*x.get_unchecked(i * 2), *x.get_unchecked(i * 2 + 1));
            let (r2, i2) = (*y.get_unchecked(i * 2), *y.get_unchecked(i * 2 + 1));
            *x.get_unchecked_mut(i * 2) = r1 * r2 - i1 * i2;
            *x.get_unchecked_mut(i * 2 + 1) = r1 * i2 + r2 * i1;
        }
    }
}

/// Perform convolution given two data serieses in the half complex format in
/// the frequency domain.
pub fn spectrum_convolve_additive(to: &mut [f32], x: &[f32], y: &[f32]) {
    assert_eq!(to.len(), x.len());
    assert_eq!(x.len(), y.len());

    // A (cyclic) convolution in the time domain can be accomplished by the
    // pointwise product in the frequency domain.
    to[0] += x[0] * y[0];
    to[1] += x[1] * y[1];

    unsafe {
        for i in 1..to.len() / 2 {
            let (r1, i1) = (*x.get_unchecked(i * 2), *x.get_unchecked(i * 2 + 1));
            let (r2, i2) = (*y.get_unchecked(i * 2), *y.get_unchecked(i * 2 + 1));
            *to.get_unchecked_mut(i * 2) += r1 * r2 - i1 * i2;
            *to.get_unchecked_mut(i * 2 + 1) += r1 * i2 + r2 * i1;
        }
    }
}
