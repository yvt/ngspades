//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {ScalarMode, SimdMode};

/// Kernels that apply a function on an interleaved array of `[u8; 4]`s.
pub trait MapU8x4InplaceKernel {
    fn apply<M: SimdMode>(&self, x: [M::U8; 4]) -> [M::U8; 4];
}

/// Extension trait for `MapU8x4InplaceKernel`.
pub trait MapU8x4InplaceKernelExt: MapU8x4InplaceKernel {
    /// Run a mapping kernel on a given slice.
    fn dispatch(&self, slice: &mut [u8]) {
        self.dispatch_simd16_unaligned(slice) || self.dispatch_scalar(slice);
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
    #[cfg(target_feature = "sse2")]
    fn dispatch_simd16_unaligned(&self, slice: &mut [u8]) -> bool {
        use simd16;
        use stdsimd::{simd, vendor};

        let mut p = slice.as_mut_ptr();
        let mut i = 0;
        while i + 63 < slice.len() {
            unsafe {
                // [ 3d 2d 1d 0d ] [ 3c 2c 1c 0c ] [ 3b 2b 1b 0b ] [ 3a 2a 1a 0a ]
                let a0 = vendor::_mm_loadu_si128(p as *const simd::__m128i).into(); // 3210 dcba
                let a1 = vendor::_mm_loadu_si128(p.offset(16) as *const simd::__m128i).into(); // 3210 hgfe
                let a2 = vendor::_mm_loadu_si128(p.offset(32) as *const simd::__m128i).into(); // 3210 lkji
                let a3 = vendor::_mm_loadu_si128(p.offset(48) as *const simd::__m128i).into(); // 3210 ponm

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

                vendor::_mm_storeu_si128(p as *mut simd::__m128i, j0);
                vendor::_mm_storeu_si128(p.offset(16) as *mut simd::__m128i, j1);
                vendor::_mm_storeu_si128(p.offset(32) as *mut simd::__m128i, j2);
                vendor::_mm_storeu_si128(p.offset(48) as *mut simd::__m128i, j3);

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
}

impl<T: MapU8x4InplaceKernel + ?Sized> MapU8x4InplaceKernelExt for T {}
