//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Fast convolution on data sequences in the frequency domain and the half
//! complex format.

#[allow(unused_imports)]
use std::ptr;
#[allow(unused_imports)]
use simdutils;

/// Perform convolution given two data serieses in the half complex format in
/// the frequency domain.
pub fn spectrum_convolve_inplace(x: &mut [f32], y: &[f32]) {
    if spectrum_convolve_inplace_avx(x, y) {
        return;
    }

    if spectrum_convolve_inplace_sse3(x, y) {
        return;
    }

    spectrum_convolve_inplace_slow(x, y);
}

fn spectrum_convolve_inplace_slow(x: &mut [f32], y: &[f32]) {
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

#[cfg(target_feature = "avx")]
fn spectrum_convolve_inplace_avx(x: &mut [f32], y: &[f32]) -> bool {
    assert_eq!(x.len(), y.len());

    if x.len() % 8 != 0 || x.len() == 8 {
        return false;
    }

    spectrum_convolve_inplace_slow(&mut x[0..8], &y[0..8]);

    let mut i = 1;
    let count = x.len() / 8;
    let xp = x.as_mut_ptr() as *mut simdutils::f32x8;
    let yp = y.as_ptr() as *const simdutils::f32x8;
    while i < count {
        unsafe {
            let xv = ptr::read_unaligned(xp.offset(i as isize));
            let yv = ptr::read_unaligned(yp.offset(i as isize));
            let zv = simdutils::avx_f32x8_complex_mul_riri(xv, yv);
            ptr::write_unaligned(xp.offset(i as isize), zv);
        }
        i += 1;
    }

    true
}

#[cfg(not(target_feature = "avx"))]
fn spectrum_convolve_inplace_avx(_: &mut [f32], _: &[f32]) -> bool {
    false
}

#[cfg(target_feature = "sse3")]
fn spectrum_convolve_inplace_sse3(x: &mut [f32], y: &[f32]) -> bool {
    assert_eq!(x.len(), y.len());

    if x.len() % 4 != 0 || x.len() == 4 {
        return false;
    }

    spectrum_convolve_inplace_slow(&mut x[0..4], &y[0..4]);

    let mut i = 1;
    let count = x.len() / 4;
    let xp = x.as_mut_ptr() as *mut simdutils::f32x4;
    let yp = y.as_ptr() as *const simdutils::f32x4;
    while i < count {
        unsafe {
            let xv = ptr::read_unaligned(xp.offset(i as isize));
            let yv = ptr::read_unaligned(yp.offset(i as isize));
            let zv = simdutils::sse3_f32x4_complex_mul_riri(xv, yv);
            ptr::write_unaligned(xp.offset(i as isize), zv);
        }
        i += 1;
    }

    true
}

#[cfg(not(target_feature = "sse3"))]
fn spectrum_convolve_inplace_sse3(_: &mut [f32], _: &[f32]) -> bool {
    false
}

/// Perform convolution given two data serieses in the half complex format in
/// the frequency domain.
///
/// A (cyclic) convolution in the time domain can be accomplished by the
/// pointwise product in the frequency domain.
pub fn spectrum_convolve_additive(to: &mut [f32], x: &[f32], y: &[f32], gain: f32) {
    if spectrum_convolve_additive_avx(to, x, y, gain) {
        return;
    }

    if spectrum_convolve_additive_sse3(to, x, y, gain) {
        return;
    }

    spectrum_convolve_additive_slow(to, x, y, gain);
}

fn spectrum_convolve_additive_slow(to: &mut [f32], x: &[f32], y: &[f32], gain: f32) {
    assert_eq!(to.len(), x.len());
    assert_eq!(x.len(), y.len());

    // Try the greater index first so the bounds checking is done only once.
    to[1] += x[1] * y[1] * gain;
    to[0] += x[0] * y[0] * gain;

    unsafe {
        if gain == 1.0 {
            for i in 1..to.len() / 2 {
                let (r1, i1) = (*x.get_unchecked(i * 2), *x.get_unchecked(i * 2 + 1));
                let (r2, i2) = (*y.get_unchecked(i * 2), *y.get_unchecked(i * 2 + 1));
                *to.get_unchecked_mut(i * 2) += r1 * r2 - i1 * i2;
                *to.get_unchecked_mut(i * 2 + 1) += r1 * i2 + r2 * i1;
            }
        } else {
            for i in 1..to.len() / 2 {
                let (r1, i1) = (*x.get_unchecked(i * 2), *x.get_unchecked(i * 2 + 1));
                let (r2, i2) = (*y.get_unchecked(i * 2), *y.get_unchecked(i * 2 + 1));
                *to.get_unchecked_mut(i * 2) += (r1 * r2 - i1 * i2) * gain;
                *to.get_unchecked_mut(i * 2 + 1) += (r1 * i2 + r2 * i1) * gain;
            }
        }
    }
}

#[cfg(target_feature = "avx")]
fn spectrum_convolve_additive_avx(to: &mut [f32], x: &[f32], y: &[f32], gain: f32) -> bool {
    assert_eq!(to.len(), x.len());
    assert_eq!(x.len(), y.len());

    if x.len() % 8 != 0 || x.len() == 8 {
        return false;
    }

    spectrum_convolve_additive_slow(&mut to[0..8], &x[0..8], &y[0..8], gain);

    let tp = to.as_mut_ptr() as *mut simdutils::f32x8;
    let xp = x.as_ptr() as *const simdutils::f32x8;
    let yp = y.as_ptr() as *const simdutils::f32x8;
    unsafe {
        if gain == 1.0 {
            for i in 1..x.len() / 8 {
                let xv = ptr::read_unaligned(xp.offset(i as isize));
                let yv = ptr::read_unaligned(yp.offset(i as isize));
                let zv = simdutils::avx_f32x8_complex_mul_riri(xv, yv);
                let tv = ptr::read_unaligned(tp.offset(i as isize));
                ptr::write_unaligned(tp.offset(i as isize), tv + zv);
            }
        } else {
            let gain = simdutils::f32x8::splat(gain);
            for i in 1..x.len() / 8 {
                let xv = ptr::read_unaligned(xp.offset(i as isize));
                let yv = ptr::read_unaligned(yp.offset(i as isize));
                let zv = simdutils::avx_f32x8_complex_mul_riri(xv, yv) * gain;
                let tv = ptr::read_unaligned(tp.offset(i as isize));
                ptr::write_unaligned(tp.offset(i as isize), tv + zv);
            }
        }
    }

    true
}

#[cfg(not(target_feature = "avx"))]
fn spectrum_convolve_additive_avx(_: &mut [f32], _: &[f32], _: &[f32], _: f32) -> bool {
    false
}

#[cfg(target_feature = "sse3")]
fn spectrum_convolve_additive_sse3(to: &mut [f32], x: &[f32], y: &[f32], gain: f32) -> bool {
    assert_eq!(to.len(), x.len());
    assert_eq!(x.len(), y.len());

    if x.len() % 4 != 0 || x.len() == 4 {
        return false;
    }

    spectrum_convolve_additive_slow(&mut to[0..4], &x[0..4], &y[0..4], gain);

    let tp = to.as_mut_ptr() as *mut simdutils::f32x4;
    let xp = x.as_ptr() as *const simdutils::f32x4;
    let yp = y.as_ptr() as *const simdutils::f32x4;
    unsafe {
        if gain == 1.0 {
            for i in 1..x.len() / 4 {
                let xv = ptr::read_unaligned(xp.offset(i as isize));
                let yv = ptr::read_unaligned(yp.offset(i as isize));
                let zv = simdutils::sse3_f32x4_complex_mul_riri(xv, yv);
                let tv = ptr::read_unaligned(tp.offset(i as isize));
                ptr::write_unaligned(tp.offset(i as isize), tv + zv);
            }
        } else {
            let gain = simdutils::f32x4::splat(gain);
            for i in 1..x.len() / 4 {
                let xv = ptr::read_unaligned(xp.offset(i as isize));
                let yv = ptr::read_unaligned(yp.offset(i as isize));
                let zv = simdutils::sse3_f32x4_complex_mul_riri(xv, yv) * gain;
                let tv = ptr::read_unaligned(tp.offset(i as isize));
                ptr::write_unaligned(tp.offset(i as isize), tv + zv);
            }
        }
    }

    true
}

#[cfg(not(target_feature = "sse3"))]
fn spectrum_convolve_additive_sse3(_: &mut [f32], _: &[f32], _: &[f32], _: f32) -> bool {
    false
}
