//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

pub use simd::{Simd, f32x4, i32x4, u32x4};
#[cfg(target_feature = "sse3")]
use simd::x86::sse3::Sse3F32x4;
#[cfg(target_feature = "avx")]
pub use simd::x86::avx::{i32x8, f32x8, u32x8, AvxF32x8};
use std::mem;

#[cfg(test)]
use num_complex::Complex;

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
extern "platform-intrinsic" {
    pub fn simd_shuffle2<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 2]) -> U;
    pub fn simd_shuffle4<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 4]) -> U;
    pub fn simd_shuffle8<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 8]) -> U;
    pub fn simd_shuffle16<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 16]) -> U;
}

#[test]
fn test_f32x4_shuffle() {
    let x = f32x4::new(1f32, 2f32, 3f32, 4f32);
    let y = f32x4::new(5f32, 6f32, 7f32, 8f32);
    assert_eq!(
        f32x4_to_array(f32x4_shuffle!(x, y, [0, 1, 2, 3])),
        [1f32, 2f32, 3f32, 4f32]
    );
    assert_eq!(
        f32x4_to_array(f32x4_shuffle!(x, y, [0, 1, 4, 5])),
        [1f32, 2f32, 5f32, 6f32]
    );
    assert_eq!(
        f32x4_to_array(f32x4_shuffle!(x, y, [2, 3, 6, 7])),
        [3f32, 4f32, 7f32, 8f32]
    );
}

#[allow(dead_code)]
#[inline]
pub fn f32x4_bitxor(lhs: f32x4, rhs: f32x4) -> f32x4 {
    let x2: i32x4 = unsafe { mem::transmute(lhs) };
    let y2: i32x4 = unsafe { mem::transmute(rhs) };
    let z = x2 ^ y2;
    unsafe { mem::transmute(z) }
}

#[allow(dead_code)]
pub fn f32x4_to_array(x: f32x4) -> [f32; 4] {
    [x.extract(0), x.extract(1), x.extract(2), x.extract(3)]
}

/// `neg_mask` must be `[0x80000000, 0x80000000, 0, 0]`
#[inline]
pub fn f32x4_complex_mul_rrii(x: f32x4, y: f32x4, neg_mask: f32x4) -> f32x4 {
    let y_iirr = f32x4_shuffle!(y, y, [2, 3, 4, 5]);

    // (y1a.r * ta.r, y1b.r * tb.r, y1a.i * ta.i, y1b.i * tb.i)
    let t2 = x * y;

    // (y1a.r * ta.i, y1b.r * tb.i, y1a.i * ta.r, y1b.i * tb.r)
    let t3 = x * y_iirr;

    // (y1a.r * ta.r, y1b.r * tb.r, y1a.r * ta.i, y1b.r * tb.i)
    let t4 = f32x4_shuffle!(t2, t3, [0, 1, 4, 5]);

    // (y1a.i * ta.i, y1b.i * tb.i, y1a.i * ta.r, y1b.i * tb.r)
    let t5 = f32x4_shuffle!(t2, t3, [2, 3, 6, 7]);

    // (-y1a.i * ta.i, -y1b.i * tb.i, y1a.i * ta.r, y1b.i * tb.r)
    let t6 = f32x4_bitxor(t5, neg_mask);

    // (y3a.r, y3b.r, y3a.i, y3b.i) =
    // (y1a.r * ta.r - y1a.i * ta.i, y1b.r * tb.r - y1b.i * tb.i,
    //  y1a.r * ta.i + y1a.i * ta.r, y1b.r * tb.i + y1b.i * tb.r)
    t4 + t6
}

#[test]
fn test_f32x4_complex_mul_rrii() {
    let neg_mask = unsafe { mem::transmute(u32x4::new(0x80000000, 0x80000000, 0, 0)) };

    let c1: Complex<f32> = Complex::new(123f32, 456f32);
    let c2: Complex<f32> = Complex::new(789f32, 135f32);
    let c3: Complex<f32> = Complex::new(114f32, 514f32);
    let c4: Complex<f32> = Complex::new(987f32, 654f32);

    let d1 = c1 * c3;
    let d2 = c2 * c4;

    let x = f32x4::new(c1.re, c2.re, c1.im, c2.im);
    let y = f32x4::new(c3.re, c4.re, c3.im, c4.im);
    let z = f32x4_complex_mul_rrii(x, y, neg_mask);

    assert_eq!(f32x4_to_array(z), [d1.re, d2.re, d1.im, d2.im]);
}

/// See `sse3_f32x4_complex_mul_riri` for usage.
#[cfg(target_feature = "sse3")]
#[inline]
pub fn sse3_f32x4_complex_mul_riri_inner(x: f32x4, y1: f32x4, y2: f32x4) -> f32x4 {
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

#[cfg(target_feature = "sse3")]
#[test]
#[allow(dead_code)]
fn test_sse3_f32x4_complex_mul_riri() {
    let c1: Complex<f32> = Complex::new(123f32, 456f32);
    let c2: Complex<f32> = Complex::new(789f32, 135f32);
    let c3: Complex<f32> = Complex::new(114f32, 514f32);
    let c4: Complex<f32> = Complex::new(987f32, 654f32);

    let d1 = c1 * c3;
    let d2 = c2 * c4;

    let x = f32x4::new(c1.re, c1.im, c2.re, c2.im);
    let y = f32x4::new(c3.re, c3.im, c4.re, c4.im);
    let z = sse3_f32x4_complex_mul_riri(x, y);

    assert_eq!(f32x4_to_array(z), [d1.re, d1.im, d2.re, d2.im]);
}

/// See `avx_f32x8_complex_mul_riri` for usage.
#[cfg(target_feature = "avx")]
#[inline]
pub fn avx_f32x8_complex_mul_riri_inner(x: f32x8, y1: f32x8, y2: f32x8) -> f32x8 {
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

#[cfg(target_feature = "avx")]
#[test]
#[allow(dead_code)]
fn test_avx_f32x8_complex_mul_riri() {
    let c1: Complex<f32> = Complex::new(123f32, 456f32);
    let c2: Complex<f32> = Complex::new(789f32, 135f32);
    let c3: Complex<f32> = Complex::new(114f32, 514f32);
    let c4: Complex<f32> = Complex::new(987f32, 654f32);
    let c5: Complex<f32> = Complex::new(12f32, 46f32);
    let c6: Complex<f32> = Complex::new(78f32, 15f32);
    let c7: Complex<f32> = Complex::new(11f32, 54f32);
    let c8: Complex<f32> = Complex::new(98f32, 64f32);

    let d1 = c1 * c3;
    let d2 = c2 * c4;
    let d3 = c5 * c7;
    let d4 = c6 * c8;

    let x = f32x8::new(c1.re, c1.im, c2.re, c2.im, c5.re, c5.im, c6.re, c6.im);
    let y = f32x8::new(c3.re, c3.im, c4.re, c4.im, c7.re, c7.im, c8.re, c8.im);
    let z = avx_f32x8_complex_mul_riri(x, y);

    assert_eq!(
        f32x8_to_array(z),
        [d1.re, d1.im, d2.re, d2.im, d3.re, d3.im, d4.re, d4.im]
    );
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
#[allow(dead_code)]
pub fn f32x8_to_array(x: f32x8) -> [f32; 8] {
    [
        x.extract(0),
        x.extract(1),
        x.extract(2),
        x.extract(3),
        x.extract(4),
        x.extract(5),
        x.extract(6),
        x.extract(7),
    ]
}
