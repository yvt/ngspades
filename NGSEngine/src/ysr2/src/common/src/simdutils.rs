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

#[cfg(target_feature = "sse3")]
#[inline]
fn sse3_f32x4_complex_mul_riri_inner(x: f32x4, y1: f32x4, y2: f32x4) -> f32x4 {
    // (r1, i1, ...) * (r3, i3, ...)
    //   --> (r1 * r3 - i1 * i3, r1 * i3 + i1 * r3, ...)

    // (r1 * r3, -i1 * i3, ...)
    let t1 = x * y1;

    // (r1 * i3, i1 * r3, ...)
    let t2 = x * y2;

    // (r1 * r3 - i1 * i3, ..., r1 * i3 + i1 * r3, ...)
    let t3 = t1.hadd(t2);

    f32x4_shuffle!(t3, t3, [0, 2, 5, 7])
}

#[cfg(target_feature = "sse3")]
#[inline]
#[allow(dead_code)]
pub fn sse3_f32x4_complex_mul_riri(x: f32x4, y: f32x4) -> f32x4 {
    let neg_mask: f32x4 = unsafe { mem::transmute(u32x4::new(0, 0x80000000, 0, 0x80000000)) };

    sse3_f32x4_complex_mul_riri_inner(
        x,
        f32x4_bitxor(y, neg_mask),
        f32x4_shuffle!(y, y, [1, 0, 7, 6]),
    )
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

#[cfg(target_feature = "avx")]
#[inline]
fn avx_f32x8_complex_mul_riri_inner(x: f32x8, y1: f32x8, y2: f32x8) -> f32x8 {
    // (r1, i1, ...) * (r3, i3, ...)
    //   --> (r1 * r3 - i1 * i3, r1 * i3 + i1 * r3, ...)

    // (r1 * r3, -i1 * i3, ...)
    let t1 = x * y1;

    // (r1 * i3, i1 * r3, ...)
    let t2 = x * y2;

    // (r1 * r3 - i1 * i3, ..., r1 * i3 + i1 * r3, ...)
    let t3 = t1.hadd(t2);

    f32x8_shuffle!(t3, t3, [0, 2, 9, 11, 4, 6, 13, 15])
}

#[cfg(target_feature = "avx")]
#[inline]
#[allow(dead_code)]
pub fn avx_f32x8_complex_mul_riri(x: f32x8, y: f32x8) -> f32x8 {
    let neg_mask: f32x8 = unsafe {
        mem::transmute(u32x8::new(
            0,
            0x80000000,
            0,
            0x80000000,
            0,
            0x80000000,
            0,
            0x80000000,
        ))
    };

    avx_f32x8_complex_mul_riri_inner(
        x,
        avx_f32x8_bitxor(y, neg_mask),
        f32x8_shuffle!(y, y, [1, 0, 11, 10, 5, 4, 15, 14]),
    )
}
