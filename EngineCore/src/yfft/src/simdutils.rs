//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[cfg(target_feature = "avx")]
pub use simd::x86::avx::{f32x8, i32x8, u32x8, u64x4, AvxF32x8};
#[cfg(target_feature = "sse2")]
pub use simd::x86::sse2::{f64x2, u64x2};
#[cfg(target_feature = "sse3")]
#[allow(unused_imports)]
use simd::x86::sse3::Sse3F32x4;
pub use simd::{f32x4, i32x4, u32x4, Simd};
use std::mem;

#[cfg(test)]
use num_complex::Complex;

/// Shuffles `u64x2` elements.
#[allow(unused_macros)]
macro_rules! u64x2_shuffle {
    ($x:expr, $y:expr, $idx:expr) => {
        unsafe {
            $crate::simdutils::simd_shuffle2::<$crate::simdutils::u64x2, $crate::simdutils::u64x2>(
                $x, $y, $idx,
            )
        }
    };
}

/// Shuffles `f64x2` elements.
#[allow(unused_macros)]
macro_rules! f64x2_shuffle {
    ($x:expr, $y:expr, $idx:expr) => {
        unsafe {
            $crate::simdutils::simd_shuffle2::<$crate::simdutils::f64x2, $crate::simdutils::f64x2>(
                $x, $y, $idx,
            )
        }
    };
}

/// Shuffles `f32x4` elements.
macro_rules! f32x4_shuffle {
    ($x:expr, $y:expr, $idx:expr) => {
        unsafe {
            $crate::simdutils::simd_shuffle4::<$crate::simdutils::f32x4, $crate::simdutils::f32x4>(
                $x, $y, $idx,
            )
        }
    };
}

/// Shuffles `f32x8` elements.
#[allow(unused_macros)]
macro_rules! f32x8_shuffle {
    ($x:expr, $y:expr, $idx:expr) => {
        unsafe {
            $crate::simdutils::simd_shuffle8::<$crate::simdutils::f32x8, $crate::simdutils::f32x8>(
                $x, $y, $idx,
            )
        }
    };
}

/// Shuffles `u64x4` elements.
#[allow(unused_macros)]
macro_rules! u64x4_shuffle {
    ($x:expr, $y:expr, $idx:expr) => {
        unsafe {
            $crate::simdutils::simd_shuffle4::<$crate::simdutils::u64x4, $crate::simdutils::u64x4>(
                $x, $y, $idx,
            )
        }
    };
}

#[allow(dead_code)]
extern "platform-intrinsic" {
    pub fn simd_shuffle2<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 2]) -> U;
    pub fn simd_shuffle4<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 4]) -> U;
    pub fn simd_shuffle8<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 8]) -> U;
    pub fn simd_shuffle16<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 16]) -> U;
    #[cfg(all(target_feature = "fma"))]
    pub fn x86_mm_fmaddsub_ps(x: f32x4, y: f32x4, z: f32x4) -> f32x4;
    #[cfg(all(target_feature = "fma", target_feature = "avx"))]
    pub fn x86_mm256_fmadd_ps(x: f32x8, y: f32x8, z: f32x8) -> f32x8;
    #[cfg(all(target_feature = "fma", target_feature = "avx"))]
    pub fn x86_mm256_fmsub_ps(x: f32x8, y: f32x8, z: f32x8) -> f32x8;
    #[cfg(all(target_feature = "fma", target_feature = "avx"))]
    pub fn x86_mm256_fmaddsub_ps(x: f32x8, y: f32x8, z: f32x8) -> f32x8;
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

#[cfg(all(target_feature = "sse3", not(target_feature = "fma")))]
#[allow(dead_code)]
pub fn sse3_fma_f32x4_fmaddsub(x: f32x4, y: f32x4, z: f32x4) -> f32x4 {
    (x * y).addsub(z)
}

#[cfg(all(target_feature = "sse3", target_feature = "fma"))]
#[allow(dead_code)]
pub fn sse3_fma_f32x4_fmaddsub(x: f32x4, y: f32x4, z: f32x4) -> f32x4 {
    unsafe { x86_mm_fmaddsub_ps(x, y, z) }
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

#[cfg(all(target_feature = "avx", not(target_feature = "fma")))]
#[allow(dead_code)]
pub fn avx_fma_f32x8_fmadd(x: f32x8, y: f32x8, z: f32x8) -> f32x8 {
    x * y + z
}

#[cfg(all(target_feature = "avx", target_feature = "fma"))]
#[allow(dead_code)]
pub fn avx_fma_f32x8_fmadd(x: f32x8, y: f32x8, z: f32x8) -> f32x8 {
    unsafe { x86_mm256_fmadd_ps(x, y, z) }
}

#[cfg(all(target_feature = "avx", not(target_feature = "fma")))]
#[allow(dead_code)]
pub fn avx_fma_f32x8_fmsub(x: f32x8, y: f32x8, z: f32x8) -> f32x8 {
    x * y - z
}

#[cfg(all(target_feature = "avx", target_feature = "fma"))]
#[allow(dead_code)]
pub fn avx_fma_f32x8_fmsub(x: f32x8, y: f32x8, z: f32x8) -> f32x8 {
    unsafe { x86_mm256_fmsub_ps(x, y, z) }
}

#[cfg(all(target_feature = "avx", not(target_feature = "fma")))]
#[allow(dead_code)]
pub fn avx_fma_f32x8_fmaddsub(x: f32x8, y: f32x8, z: f32x8) -> f32x8 {
    (x * y).addsub(z)
}

#[cfg(all(target_feature = "avx", target_feature = "fma"))]
#[allow(dead_code)]
pub fn avx_fma_f32x8_fmaddsub(x: f32x8, y: f32x8, z: f32x8) -> f32x8 {
    unsafe { x86_mm256_fmaddsub_ps(x, y, z) }
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
