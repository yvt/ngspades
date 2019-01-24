//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

// Copied from yFFT's simdutils.rs

#[cfg(target_arch = "x86")]
use std::arch::x86 as vendor;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64 as vendor;
pub use packed_simd::{f32x4, i32x4, u32x4, i32x8, f32x8, u32x8};
use std::mem;

#[allow(dead_code)]
#[inline]
pub fn f32x4_bitxor(lhs: f32x4, rhs: f32x4) -> f32x4 {
    let x2: i32x4 = unsafe { mem::transmute(lhs) };
    let y2: i32x4 = unsafe { mem::transmute(rhs) };
    let z = x2 ^ y2;
    unsafe { mem::transmute(z) }
}

#[cfg(all(target_feature = "sse3", not(target_feature = "fma")))]
#[allow(dead_code)]
pub fn sse3_fma_f32x4_fmaddsub(x: f32x4, y: f32x4, z: f32x4) -> f32x4 {
    let xy = unsafe { mem::transmute(x * y) };
    let z = unsafe { mem::transmute(z) };
    let w = unsafe { vendor::_mm_addsub_ps(xy, z) };
    unsafe { mem::transmute(w) }
}

#[cfg(all(target_feature = "sse3", target_feature = "fma"))]
#[allow(dead_code)]
pub fn sse3_fma_f32x4_fmaddsub(x: f32x4, y: f32x4, z: f32x4) -> f32x4 {
    let x = unsafe { mem::transmute(x) };
    let y = unsafe { mem::transmute(y) };
    let z = unsafe { mem::transmute(z) };
    let w = unsafe { vendor::_mm_fmaddsub_ps(x, y, z) };
    unsafe { mem::transmute(w) }
}

#[cfg(target_feature = "sse3")]
#[inline]
#[allow(dead_code)]
pub fn sse3_f32x4_complex_mul_riri(x: f32x4, y: f32x4) -> f32x4 {
    // (r1, i1, ...) * (r3, i3, ...)
    //   --> ((r1 * r3) - (i1 * i3), (r1 * i3) + (i1 * r3), ...)
    let x1: f32x4 = shuffle!(x, x, [0, 0, 2, 2]); // movsldup
    let x2: f32x4 = shuffle!(x, x, [1, 1, 3, 3]); // movshdup
    let y1: f32x4 = y;
    let y2: f32x4 = shuffle!(y, y, [1, 0, 3, 2]); // shufps
    let z = sse3_fma_f32x4_fmaddsub(x1, y1, x2 * y2); // vaddsubps/vfmaddsubXXXps
    return z;
}

#[cfg(target_feature = "avx")]
#[allow(dead_code)]
#[inline]
pub fn avx_f32x8_bitxor(lhs: f32x8, rhs: f32x8) -> f32x8 {
    let x2: i32x8 = unsafe { mem::transmute(lhs) };
    let y2: i32x8 = unsafe { mem::transmute(rhs) };
    let z = x2 ^ y2;
    unsafe { mem::transmute(z) }
}

#[cfg(all(target_feature = "avx", not(target_feature = "fma")))]
#[allow(dead_code)]
pub fn avx_fma_f32x8_fmaddsub(x: f32x8, y: f32x8, z: f32x8) -> f32x8 {
    let xy: vendor::__m256 = unsafe { mem::transmute(x * y) };
    let z: vendor::__m256 = unsafe { mem::transmute(z) };
    unsafe { mem::transmute(vendor::_mm256_addsub_ps(xy, z)) }
}

#[cfg(all(target_feature = "avx", target_feature = "fma"))]
#[allow(dead_code)]
pub fn avx_fma_f32x8_fmaddsub(x: f32x8, y: f32x8, z: f32x8) -> f32x8 {
    let x: vendor::__m256 = unsafe { mem::transmute(x) };
    let y: vendor::__m256 = unsafe { mem::transmute(y) };
    let z: vendor::__m256 = unsafe { mem::transmute(z) };
    unsafe { mem::transmute(vendor::_mm256_fmaddsub_ps(x, y, z)) }
}

#[cfg(target_feature = "avx")]
#[inline]
#[allow(dead_code)]
pub fn avx_f32x8_complex_mul_riri(x: f32x8, y: f32x8) -> f32x8 {
    // (r1, i1, ...) * (r3, i3, ...)
    //   --> ((r1 * r3) - (i1 * i3), (r1 * i3) + (i1 * r3), ...)
    let x1: f32x8 = shuffle!(x, x, [0, 0, 2, 2, 4, 4, 6, 6]); // vmovsldup
    let x2: f32x8 = shuffle!(x, x, [1, 1, 3, 3, 5, 5, 7, 7]); // vmovshdup
    let y1: f32x8 = y;
    let y2: f32x8 = shuffle!(y, y, [1, 0, 3, 2, 5, 4, 7, 6]); // vpermilps
    let z = avx_fma_f32x8_fmaddsub(x1, y1, x2 * y2); // vaddsubps/vfmaddsubXXXps
    return z;
}
