//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

// Copied from yFFT's simdutils.rs

pub use simd::{Simd, f32x4, i32x4, u32x4};
#[cfg(target_feature = "sse3")]
pub use simd::x86::sse3::Sse3F32x4;
#[cfg(target_feature = "avx")]
pub use simd::x86::avx::{i32x8, f32x8, u32x8, AvxF32x8};
use std::mem;

#[allow(dead_code)]
extern "platform-intrinsic" {
    pub fn simd_shuffle2<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 2]) -> U;
    pub fn simd_shuffle4<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 4]) -> U;
    pub fn simd_shuffle8<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 8]) -> U;
    pub fn simd_shuffle16<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 16]) -> U;
    #[cfg(all(target_feature = "fma"))]
    pub fn x86_mm_fmaddsub_ps(x: f32x4, y: f32x4, z: f32x4) -> f32x4;
    #[cfg(all(target_feature = "fma", target_feature = "avx"))]
    pub fn x86_mm256_fmaddsub_ps(x: f32x8, y: f32x8, z: f32x8) -> f32x8;
}

/// Shuffles `f32x4` elements.
macro_rules! f32x4_shuffle {
    ($x:expr, $y:expr, $idx:expr) => {
        unsafe { $crate::simdutils::simd_shuffle4::<$crate::simdutils::f32x4, $crate::simdutils::f32x4>($x, $y, $idx) }
    }
}

/// Shuffles `f32x8` elements.
#[allow(unused_macros)]
macro_rules! f32x8_shuffle {
    ($x:expr, $y:expr, $idx:expr) => {
        unsafe { $crate::simdutils::simd_shuffle8::<$crate::simdutils::f32x8, $crate::simdutils::f32x8>($x, $y, $idx) }
    }
}

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
    (x * y).addsub(z)
}

#[cfg(all(target_feature = "sse3", target_feature = "fma"))]
#[allow(dead_code)]
pub fn sse3_fma_f32x4_fmaddsub(x: f32x4, y: f32x4, z: f32x4) -> f32x4 {
    unsafe {
        x86_mm_fmaddsub_ps(x, y, z)
    }
}

#[cfg(target_feature = "sse3")]
#[inline]
#[allow(dead_code)]
pub fn sse3_f32x4_complex_mul_riri(x: f32x4, y: f32x4) -> f32x4 {
    // (r1, i1, ...) * (r3, i3, ...)
    //   --> ((r1 * r3) - (i1 * i3), (r1 * i3) + (i1 * r3), ...)
    let x1 = f32x4_shuffle!(x, x, [0, 0, 2, 2]); // movsldup
    let x2 = f32x4_shuffle!(x, x, [1, 1, 3, 3]); // movshdup
    let y1 = y;
    let y2 = f32x4_shuffle!(y, y, [1, 0, 3, 2]); // shufps
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
    (x * y).addsub(z)
}

#[cfg(all(target_feature = "avx", target_feature = "fma"))]
#[allow(dead_code)]
pub fn avx_fma_f32x8_fmaddsub(x: f32x8, y: f32x8, z: f32x8) -> f32x8 {
    unsafe {
        x86_mm256_fmaddsub_ps(x, y, z)
    }
}

#[cfg(target_feature = "avx")]
#[inline]
#[allow(dead_code)]
pub fn avx_f32x8_complex_mul_riri(x: f32x8, y: f32x8) -> f32x8 {
    // (r1, i1, ...) * (r3, i3, ...)
    //   --> ((r1 * r3) - (i1 * i3), (r1 * i3) + (i1 * r3), ...)
    let x1 = f32x8_shuffle!(x, x, [0, 0, 2, 2, 4, 4, 6, 6]); // vmovsldup
    let x2 = f32x8_shuffle!(x, x, [1, 1, 3, 3, 5, 5, 7, 7]); // vmovshdup
    let y1 = y;
    let y2 = f32x8_shuffle!(y, y, [1, 0, 3, 2, 5, 4, 7, 6]); // vpermilps
    let z = avx_fma_f32x8_fmaddsub(x1, y1, x2 * y2); // vaddsubps/vfmaddsubXXXps
    return z;
}
