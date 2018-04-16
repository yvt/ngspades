//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![allow(unused_imports)]
extern crate x86intrin;

use std::mem::transmute;
use stdsimd::simd::{__m256i, i32x8};

// A replacement for `stdsimd::vendor::_mm256_i32gather_epi32` which generates
// horrible code for some reasons.
#[inline(always)]
#[cfg(target_feature = "avx2")]
pub unsafe fn mm256_i32gather_epi32(slice: *const i32, offsets: i32x8, scale: i32) -> i32x8 {
    transmute(x86intrin::avx2::mm256_i32gather_epi32(
        slice,
        transmute(offsets),
        scale,
    ))
}

// A replacement for `stdsimd::vendor::_mm256_permute2x128_si256` which generates
// horrible code for some reasons.
#[inline(always)]
#[cfg(target_feature = "avx2")]
pub unsafe fn mm256_permute2x128_si256(a: __m256i, b: __m256i, imm8: i32) -> __m256i {
    transmute(x86intrin::avx2::mm256_permute2x128_si256(
        transmute(a),
        transmute(b),
        imm8,
    ))
}
