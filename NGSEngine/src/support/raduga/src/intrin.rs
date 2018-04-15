//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate x86intrin;

use std::mem::transmute;
use stdsimd::simd::i32x8;

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
