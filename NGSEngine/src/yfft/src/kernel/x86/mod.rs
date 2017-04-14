//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

//! Defines FFT kernels optimized for x86 and x86_64 systems.

use super::{Kernel, KernelCreationParams, KernelParams, KernelType, SliceAccessor};
use super::super::Num;
use simd::{Simd, f32x4, i32x4};
use std::mem;

use num_complex::Complex;

macro_rules! f32x4_shuffle {
    ($x:expr, $y:expr, $idx:expr) => {
        unsafe { $crate::kernel::x86::simd_shuffle4::<$crate::kernel::x86::f32x4, $crate::kernel::x86::f32x4>($x, $y, $idx) }
    }
}

mod x86sse;
mod x86sse2;
mod x86sse3;

pub fn new_x86_kernel<T>(cparams: &KernelCreationParams) -> Option<Box<Kernel<T>>>
    where T : Num {
    x86sse3::new_x86_sse3_kernel(cparams)
        .or_else(|| x86sse2::new_x86_sse2_kernel(cparams))
        .or_else(|| x86sse::new_x86_sse_kernel(cparams))
}

#[allow(dead_code)]
extern "platform-intrinsic" {
    fn simd_shuffle2<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 2]) -> U;
    fn simd_shuffle4<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 4]) -> U;
    fn simd_shuffle8<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 8]) -> U;
    fn simd_shuffle16<T: Simd, U: Simd<Elem = T::Elem>>(x: T, y: T, idx: [u32; 16]) -> U;
}

#[inline]
fn f32x4_bitxor(lhs: f32x4, rhs: f32x4) -> f32x4 {
    let x2: i32x4 = unsafe { mem::transmute(lhs) };
    let y2: i32x4 = unsafe { mem::transmute(rhs) };
    let z = x2 ^ y2;
    unsafe { mem::transmute(z) }
}

#[allow(dead_code)]
fn f32x4_to_array(x: f32x4) -> [f32; 4] {
    let mut y = [0f32; 4];
    unsafe { *(&mut y[0] as *mut f32 as *mut f32x4) = x };
    y
}

#[test]
fn test_f32x4_shuffle() {
    let x = f32x4::new(1f32, 2f32, 3f32, 4f32);
    let y = f32x4::new(5f32, 6f32, 7f32, 8f32);
    assert_eq!(f32x4_to_array(f32x4_shuffle!(x, y, [0, 1, 2, 3])), [1f32, 2f32, 3f32, 4f32]);
    assert_eq!(f32x4_to_array(f32x4_shuffle!(x, y, [0, 1, 4, 5])), [1f32, 2f32, 5f32, 6f32]);
    assert_eq!(f32x4_to_array(f32x4_shuffle!(x, y, [2, 3, 6, 7])), [3f32, 4f32, 7f32, 8f32]);
}
