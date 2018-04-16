//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![allow(unused_imports)]
use stdsimd::{simd, vendor};
use {intrin, simd16};
use {ScalarMode, SimdMode};

/// Kernels that apply a function on an interleaved array of `[u8; 4]`s.
pub trait MapU8x4InplaceKernel {
    fn apply<M: SimdMode>(&self, x: [M::U8; 4]) -> [M::U8; 4];
}

/// Extension trait for `MapU8x4InplaceKernel`.
pub trait MapU8x4InplaceKernelExt: MapU8x4InplaceKernel {
    /// Run a mapping kernel on a given slice.
    fn dispatch(&self, slice: &mut [u8]) {
        self.dispatch_simd16_masked(slice) || self.dispatch_simd16_unaligned(slice)
            || self.dispatch_scalar(slice);
    }

    #[doc(hidden)]
    fn dispatch_scalar(&self, slice: &mut [u8]) -> bool {
        let mut i = 0;
        while i + 3 < slice.len() {
            let input = [slice[i], slice[i + 1], slice[i + 2], slice[i + 3]];
            let output = self.apply::<ScalarMode>(input);
            slice[i] = output[0];
            slice[i + 1] = output[1];
            slice[i + 2] = output[2];
            slice[i + 3] = output[3];
            i += 4;
        }
        true
    }

    #[doc(hidden)]
    #[cfg(target_feature = "avx2")]
    fn dispatch_simd16_masked(&self, slice: &mut [u8]) -> bool {
        unsafe {
            // Unbenefitical on short arrays
            if slice.len() < 16 {
                return false;
            }

            // OVerflows the counter on long arrays
            if slice.len() / 4 > 0x7ffffff0 {
                return false;
            }

            // The array must be 4-byte aligned
            let addr = slice.as_ptr() as usize;
            if addr & 3 != 0 {
                return false;
            }
            let end_addr = addr + slice.len() / 4 * 4;
            let addr_aligned = addr & !15usize;

            // Array indices, scaled by 2 (to emulate `u32` comparison using `i32`s.
            // I hope you know the hack that you do `0 <= x < A` with just one comparison)
            let i_start = -(((addr - addr_aligned) / 4) as i32);
            let mut indices0 =
                simd::i32x8::new(0 * 2, 1 * 2, 2 * 2, 3 * 2, 4 * 2, 5 * 2, 6 * 2, 7 * 2)
                    + simd::i32x8::splat(i_start * 2);
            let bounds = simd::i32x8::splat((slice.len() / 4) as i32);

            let mut i = addr_aligned;
            while i < end_addr {
                let indices1 = indices0 + simd::i32x8::splat(16);

                let mask0 = vendor::_mm256_srli_epi32(indices0, 1).lt(bounds);
                let mask1 = vendor::_mm256_srli_epi32(indices1, 1).lt(bounds);

                let a0 = vendor::_mm256_maskload_epi32(i as *const _, mask0).into();
                let a1 = vendor::_mm256_maskload_epi32((i + 32) as *const _, mask1).into();

                let c = self.dispatch_simd16_m256([a0, a1]);

                vendor::_mm256_maskstore_epi32(i as *mut _, mask0, c[0].into());
                vendor::_mm256_maskstore_epi32((i + 32) as *mut _, mask1, c[1].into());

                indices0 += simd::i32x8::splat(32);
                i += 64;
            }
        }
        true
    }

    #[doc(hidden)]
    #[cfg(not(target_feature = "avx2"))]
    fn dispatch_simd16_masked(&self, _: &mut [u8]) -> bool {
        false
    }

    #[doc(hidden)]
    #[cfg(all(target_feature = "sse2", not(target_feature = "avx2")))]
    fn dispatch_simd16_unaligned(&self, slice: &mut [u8]) -> bool {
        let mut p = slice.as_mut_ptr();
        let mut i = 0;
        while i + 63 < slice.len() {
            unsafe {
                let a0 = vendor::_mm_loadu_si128(p as *const simd::__m128i).into();
                let a1 = vendor::_mm_loadu_si128(p.offset(16) as *const simd::__m128i).into();
                let a2 = vendor::_mm_loadu_si128(p.offset(32) as *const simd::__m128i).into();
                let a3 = vendor::_mm_loadu_si128(p.offset(48) as *const simd::__m128i).into();

                let f = self.dispatch_simd16_m128([a0, a1, a2, a3]);

                vendor::_mm_storeu_si128(p as *mut simd::__m128i, f[0]);
                vendor::_mm_storeu_si128(p.offset(16) as *mut simd::__m128i, f[1]);
                vendor::_mm_storeu_si128(p.offset(32) as *mut simd::__m128i, f[2]);
                vendor::_mm_storeu_si128(p.offset(48) as *mut simd::__m128i, f[3]);

                p = p.offset(64);
                i += 64;
            }
        }

        self.dispatch_scalar(&mut slice[i..]);
        true
    }

    #[doc(hidden)]
    #[cfg(target_feature = "avx2")]
    fn dispatch_simd16_unaligned(&self, slice: &mut [u8]) -> bool {
        let mut p = slice.as_mut_ptr();
        let mut i = 0;
        while i + 63 < slice.len() {
            unsafe {
                let a0 = vendor::_mm256_loadu_si256(p as *const simd::__m256i).into();
                let a1 = vendor::_mm256_loadu_si256(p.offset(32) as *const simd::__m256i).into();

                let f = self.dispatch_simd16_m256([a0, a1]);

                vendor::_mm256_storeu_si256(p as *mut simd::__m256i, f[0]);
                vendor::_mm256_storeu_si256(p.offset(32) as *mut simd::__m256i, f[1]);

                p = p.offset(64);
                i += 64;
            }
        }

        self.dispatch_scalar(&mut slice[i..]);
        true
    }

    #[doc(hidden)]
    #[cfg(not(target_feature = "sse2"))]
    fn dispatch_simd16_unaligned(&self, _: &mut [u8]) -> bool {
        false
    }

    #[doc(hidden)]
    #[cfg(target_feature = "avx2")]
    #[inline(always)]
    unsafe fn dispatch_simd16_m256(&self, a: [simd::__m256i; 2]) -> [simd::__m256i; 2] {
        // ... [ 3d 2d 1d 0d ] [ 3c 2c 1c 0c ] [ 3b 2b 1b 0b ] [ 3a 2a 1a 0a ]
        let a0 = a[0]; // hgfedcba 3210
        let a1 = a[1]; // ponmlkji 3210

        let transpose4x4 = simd::u8x32::new(
            0, 4, 8, 12, 1, 5, 9, 13, 2, 6, 10, 14, 3, 7, 11, 15, 0, 4, 8, 12, 1, 5, 9, 13, 2, 6,
            10, 14, 3, 7, 11, 15,
        );

        let b0 = vendor::_mm256_shuffle_epi8(a0.into(), transpose4x4).into(); // 3210 hgfe / 3210 dcba
        let b1 = vendor::_mm256_shuffle_epi8(a1.into(), transpose4x4).into(); // 3210 ponm / 3210 lkji

        let transpose4x2 = simd::u32x8::new(0, 4, 1, 5, 2, 6, 3, 7);

        let c0 = vendor::_mm256_permutevar8x32_epi32(b0, transpose4x2).into(); // 3210 hgfedcba
        let c1 = vendor::_mm256_permutevar8x32_epi32(b1, transpose4x2).into(); // 3210 ponmlkji

        let d0 = vendor::_mm256_unpacklo_epi64(c0, c1).into(); // 20 ponmlkjihgfedcba
        let d1 = vendor::_mm256_unpackhi_epi64(c0, c1).into(); // 31 ponmlkjihgfedcba

        let e0 = vendor::_mm256_extractf128_si256(d0, 0).into(); // 0 ponmlkjihgfedcba
        let e1 = vendor::_mm256_extractf128_si256(d1, 0).into(); // 1 ponmlkjihgfedcba
        let e2 = vendor::_mm256_extractf128_si256(d0, 1).into(); // 2 ponmlkjihgfedcba
        let e3 = vendor::_mm256_extractf128_si256(d1, 1).into(); // 3 ponmlkjihgfedcba

        let f = self.apply::<simd16::Simd16Mode>([
            simd16::Simd16U8(e0),
            simd16::Simd16U8(e1),
            simd16::Simd16U8(e2),
            simd16::Simd16U8(e3),
        ]);

        let f0 = f[0].0.into();
        let f1 = f[1].0.into();
        let f2 = f[2].0.into();
        let f3 = f[3].0.into();

        let g0 = vendor::_mm256_set_m128i(f1, f0).into(); // 10 ponmlkjihgfedcba
        let g1 = vendor::_mm256_set_m128i(f3, f2).into(); // 32 ponmlkjihgfedcba

        let h0 = vendor::_mm256_permute4x64_epi64(g0, 0b11_01_10_00).into(); // 10 ponmlkji / 10 hgfedcba
        let h1 = vendor::_mm256_permute4x64_epi64(g1, 0b11_01_10_00).into(); // 32 ponmlkji / 32 hgfedcba

        let i0 = intrin::mm256_permute2x128_si256(h0, h1, 0b0010_0000).into(); // 3210 hgfedcba
        let i1 = intrin::mm256_permute2x128_si256(h0, h1, 0b0011_0001).into(); // 3210 ponmlkji

        let transpose2x4 = simd::u32x8::new(0, 2, 4, 6, 1, 3, 5, 7);

        let j0 = vendor::_mm256_permutevar8x32_epi32(i0, transpose2x4).into(); // 3210 hgfe / 3210 dcba
        let j1 = vendor::_mm256_permutevar8x32_epi32(i1, transpose2x4).into(); // 3210 ponm / 3210 lkji

        let k0 = vendor::_mm256_shuffle_epi8(j0, transpose4x4).into(); // hgfedcba 3210
        let k1 = vendor::_mm256_shuffle_epi8(j1, transpose4x4).into(); // ponmlkji 3210

        [k0, k1]
    }

    #[doc(hidden)]
    #[cfg(target_feature = "sse2")]
    #[inline(always)]
    unsafe fn dispatch_simd16_m128(&self, x: [simd::__m128i; 4]) -> [simd::__m128i; 4] {
        // [ 3d 2d 1d 0d ] [ 3c 2c 1c 0c ] [ 3b 2b 1b 0b ] [ 3a 2a 1a 0a ]
        let a0 = x[0].into(); // dcba 3210
        let a1 = x[1].into(); // hgfe 3210
        let a2 = x[2].into(); // lkji 3210
        let a3 = x[3].into(); // ponm 3210

        // [ 3f 3b ] [ 2f 2b ] [ 1f 1b ] [ 0f 0b ] [ 3e 3a ] [ 2e 2a ] [ 1e 1a ] [ 0e 0a ]
        let b0 = vendor::_mm_unpacklo_epi8(a0, a1).into(); // 3210 fb / 3210 ea
        let b1 = vendor::_mm_unpackhi_epi8(a0, a1).into(); // 3210 hd / 3210 gc
        let b2 = vendor::_mm_unpacklo_epi8(a2, a3).into(); // 3210 nj / 3210 mi
        let b3 = vendor::_mm_unpackhi_epi8(a2, a3).into(); // 3210 pl / 3210 ok

        // [ 3g 3c 3e 3a ] [ 2g 2c 2e 2a ] [ 1g 1c 1e 1a ] [ 0g 0c 0e 0a ]
        let c0 = vendor::_mm_unpacklo_epi16(b0, b1).into(); // 3210 gcea
        let c1 = vendor::_mm_unpackhi_epi16(b0, b1).into(); // 3210 hdfb
        let c2 = vendor::_mm_unpacklo_epi16(b2, b3).into(); // 3210 okmi
        let c3 = vendor::_mm_unpackhi_epi16(b2, b3).into(); // 3210 plnj

        // [ 1h 1d 1f 1b 1g 1c 1e 1a ] [ 0h 0d 0f 0b 0g 0c 0e 0a ]
        let d0 = vendor::_mm_unpacklo_epi32(c0, c1).into(); // 10 hdfbgcea
        let d1 = vendor::_mm_unpackhi_epi32(c0, c1).into(); // 32 hdfbgcea
        let d2 = vendor::_mm_unpacklo_epi32(c2, c3).into(); // 10 plnjokmi
        let d3 = vendor::_mm_unpackhi_epi32(c2, c3).into(); // 32 plnjokmi

        // [ 0p 0l 0n 0j 0o 0k 0m 0i 0h 0d 0f 0b 0g 0c 0e 0a ]
        let e0 = vendor::_mm_unpacklo_epi64(d0, d2).into(); // 0 plnjokmihdfbgcea
        let e1 = vendor::_mm_unpackhi_epi64(d0, d2).into(); // 1 plnjokmihdfbgcea
        let e2 = vendor::_mm_unpacklo_epi64(d1, d3).into(); // 2 plnjokmihdfbgcea
        let e3 = vendor::_mm_unpackhi_epi64(d1, d3).into(); // 3 plnjokmihdfbgcea

        let f = self.apply::<simd16::Simd16Mode>([
            simd16::Simd16U8(e0),
            simd16::Simd16U8(e1),
            simd16::Simd16U8(e2),
            simd16::Simd16U8(e3),
        ]);

        let f0 = f[0].0.into();
        let f1 = f[1].0.into();
        let f2 = f[2].0.into();
        let f3 = f[3].0.into();

        // [ 1h 0h ] [ 1d 0d ] [ 1f 0f ] [ 1b 0b ] [ 1g 0g ] [ 1c 0c ] [ 1e 0e ] [ 1a 0a ]
        let g0 = vendor::_mm_unpacklo_epi8(f0, f1).into(); // hdfbgcea 10
        let g1 = vendor::_mm_unpackhi_epi8(f0, f1).into(); // plnjokmi 10
        let g2 = vendor::_mm_unpacklo_epi8(f2, f3).into(); // hdfbgcea 32
        let g3 = vendor::_mm_unpackhi_epi8(f2, f3).into(); // plnjokmi 32

        // [ 3g 2g 1g 0g ] [ 3c 2c 1c 0c ] [ 3e 2e 1e 0e ] [ 3a 2a 1a 0a ]
        let h0 = vendor::_mm_unpacklo_epi16(g0, g2).into(); // gcea 3210
        let h1 = vendor::_mm_unpackhi_epi16(g0, g2).into(); // hdfb 3210
        let h2 = vendor::_mm_unpacklo_epi16(g1, g3).into(); // okmi 3210
        let h3 = vendor::_mm_unpackhi_epi16(g1, g3).into(); // plnj 3210

        let i0 = vendor::_mm_unpacklo_epi32(h0, h1).into(); // feba 3210
        let i1 = vendor::_mm_unpackhi_epi32(h0, h1).into(); // hgdc 3210
        let i2 = vendor::_mm_unpacklo_epi32(h2, h3).into(); // nmji 3210
        let i3 = vendor::_mm_unpackhi_epi32(h2, h3).into(); // polk 3210

        let j0 = vendor::_mm_unpacklo_epi64(i0, i1).into(); // dcba 3210
        let j1 = vendor::_mm_unpackhi_epi64(i0, i1).into(); // hgfe 3210
        let j2 = vendor::_mm_unpacklo_epi64(i2, i3).into(); // lkji 3210
        let j3 = vendor::_mm_unpackhi_epi64(i2, i3).into(); // ponm 3210

        [j0, j1, j2, j3]
    }
}

impl<T: MapU8x4InplaceKernel + ?Sized> MapU8x4InplaceKernelExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use prelude::*;

    struct Xorshift32(u32);

    impl Xorshift32 {
        fn next(&mut self) -> u32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 17;
            self.0 ^= self.0 << 5;
            self.0
        }
    }

    #[test]
    fn dispatchers_agree() {
        struct Kernel;
        impl MapU8x4InplaceKernel for Kernel {
            #[inline]
            fn apply<M: SimdMode>(&self, x: [M::U8; 4]) -> [M::U8; 4] {
                [
                    x[0] + M::U8::splat(64) + x[1] + x[2] + x[3] + M::U8::splat(11),
                    x[0] + M::U8::splat(64) - x[1] + x[2] - x[3] + M::U8::splat(45),
                    x[0] + M::U8::splat(64) + x[1] - x[2] - x[3] + M::U8::splat(1),
                    x[0] + M::U8::splat(64) - x[1] - x[2] + x[3] + M::U8::splat(4),
                ]
            }
        }

        let mut state = Xorshift32(12345);
        let input: Vec<_> = (0..256).map(|_| state.next() as u8 >> 3).collect();

        for &range_start in [0, 4, 16, 17, 128].iter() {
            for &range_end in [128, 132, 255, 256].iter() {
                let range = range_start..range_end;

                println!("Range = {:?}", range);

                let mut reference = input.clone();
                Kernel.dispatch_scalar(&mut reference[range.clone()]);

                let mut result = input.clone();
                if Kernel.dispatch_simd16_masked(&mut result[range.clone()]) {
                    assert_eq!(result, reference);
                }

                let mut result = input.clone();
                if Kernel.dispatch_simd16_unaligned(&mut result[range.clone()]) {
                    assert_eq!(result, reference);
                }
            }
        }
    }
}
