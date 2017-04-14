//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

pub use simd::{Simd, f32x4, i32x4};
use std::mem;

#[cfg(test)]
use num_complex::Complex;

macro_rules! f32x4_shuffle {
    ($x:expr, $y:expr, $idx:expr) => {
        unsafe { $crate::simdutils::simd_shuffle4::<$crate::simdutils::f32x4, $crate::simdutils::f32x4>($x, $y, $idx) }
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
    assert_eq!(f32x4_to_array(f32x4_shuffle!(x, y, [0, 1, 2, 3])), [1f32, 2f32, 3f32, 4f32]);
    assert_eq!(f32x4_to_array(f32x4_shuffle!(x, y, [0, 1, 4, 5])), [1f32, 2f32, 5f32, 6f32]);
    assert_eq!(f32x4_to_array(f32x4_shuffle!(x, y, [2, 3, 6, 7])), [3f32, 4f32, 7f32, 8f32]);
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
    let mut y = [0f32; 4];
    unsafe { *(&mut y[0] as *mut f32 as *mut f32x4) = x };
    y
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
    let neg_mask_raw: [u32; 4] = [0x80000000, 0x80000000, 0, 0];
    let neg_mask = unsafe { *(&neg_mask_raw as *const u32 as *const f32x4) };

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
