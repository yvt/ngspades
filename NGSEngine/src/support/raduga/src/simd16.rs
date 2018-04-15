//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use intrin;
use stdsimd::{simd, vendor};
use {IntPacked, Packed, PackedI16, PackedU16, PackedU32, PackedU8, SimdMode};

#[derive(Debug, Copy, Clone)]
pub struct Simd16Mode;

impl SimdMode for Simd16Mode {
    type U8 = Simd16U8;
    type U16 = Simd16U16;
    type U32 = Simd16U32;

    type I16 = Simd16I16;
}

#[derive(Debug, Copy, Clone)]
pub struct Simd16U8(pub simd::u8x16);

impl_packed_ops!(Simd16U8, 0);
impl_int_packed_ops!(Simd16U8, 0);

unsafe impl Packed for Simd16U8 {
    type Scalar = u8;
    type Mode = Simd16Mode;
    const WIDTH: usize = 16;

    #[inline]
    fn table<F: FnMut(usize) -> Self::Scalar>(mut f: F) -> Self {
        Simd16U8(simd::u8x16::new(
            f(0),
            f(1),
            f(2),
            f(3),
            f(4),
            f(5),
            f(6),
            f(7),
            f(8),
            f(9),
            f(10),
            f(11),
            f(12),
            f(13),
            f(14),
            f(15),
        ))
    }
    unsafe fn get_unchecked(self, i: usize) -> Self::Scalar {
        self.0.extract_unchecked(i as u32)
    }
    fn splat(x: Self::Scalar) -> Self {
        Simd16U8(simd::u8x16::splat(x))
    }

    fn as_u8(self) -> <Self::Mode as SimdMode>::U8 {
        self
    }
    #[cfg(target_feature = "avx2")]
    fn as_u16(self) -> <Self::Mode as SimdMode>::U16 {
        Simd16U16::from_u8(self)
    }
    #[cfg(target_feature = "avx2")]
    fn as_u32(self) -> <Self::Mode as SimdMode>::U32 {
        Simd16U32::from_u8(self)
    }

    fn as_i16(self) -> <Self::Mode as SimdMode>::I16 {
        self.as_u16().as_i16()
    }

    #[inline]
    #[cfg(target_feature = "avx2")]
    unsafe fn gather32_ptr(
        base: *const Self::Scalar,
        offset: <Self::Mode as SimdMode>::U32,
        scale: u8,
    ) -> Self {
        // Load 4-byte values
        let data0 = intrin::mm256_i32gather_epi32(base as *const _, offset.0.into(), scale as i32);
        let data1 = intrin::mm256_i32gather_epi32(base as *const _, offset.1.into(), scale as i32);

        // Throw away the extra 24 MSBs
        Simd16U32(data0.into(), data1.into()).as_u8()
    }
}

impl IntPacked for Simd16U8 {}
impl PackedU8 for Simd16U8 {}

/// Implementation of SIMD types that utilizes 256-bit wide registers of AVX(2).
#[cfg(target_feature = "avx2")]
mod avx2 {
    use super::*;

    #[derive(Debug, Copy, Clone)]
    pub struct Simd16U16(pub simd::u16x16);

    impl_packed_ops!(Simd16U16, 0);
    impl_int_packed_ops!(Simd16U16, 0);

    impl Simd16U16 {
        #[inline]
        pub(super) fn from_u8(x: Simd16U8) -> Self {
            unsafe { Simd16U16(vendor::_mm256_cvtepu8_epi16(x.0.into()).into()) }
        }
    }

    unsafe impl Packed for Simd16U16 {
        type Scalar = u16;
        type Mode = Simd16Mode;
        const WIDTH: usize = 16;

        #[inline]
        fn table<F: FnMut(usize) -> Self::Scalar>(mut f: F) -> Self {
            Simd16U16(simd::u16x16::new(
                f(0),
                f(1),
                f(2),
                f(3),
                f(4),
                f(5),
                f(6),
                f(7),
                f(8),
                f(9),
                f(10),
                f(11),
                f(12),
                f(13),
                f(14),
                f(15),
            ))
        }
        unsafe fn get_unchecked(self, i: usize) -> Self::Scalar {
            self.0.extract_unchecked(i as u32)
        }
        fn splat(x: Self::Scalar) -> Self {
            Simd16U16(simd::u16x16::splat(x))
        }

        #[inline]
        fn as_u8(self) -> <Self::Mode as SimdMode>::U8 {
            unsafe {
                // Discard upper 8 bits so they don't affect the result of
                // `_mm256_packus_epi16` (which casts `i16` to `u8` with saturation)
                let u16s = (self.0 & simd::u16x16::splat(0xff)).into();

                // Split into two `m128i`s
                let lo = vendor::_mm256_extractf128_si256(u16s, 0).into();
                let hi = vendor::_mm256_extractf128_si256(u16s, 1).into();

                Simd16U8(vendor::_mm_packus_epi16(lo, hi))
            }
        }
        fn as_u16(self) -> <Self::Mode as SimdMode>::U16 {
            self
        }
        fn as_u32(self) -> <Self::Mode as SimdMode>::U32 {
            unsafe {
                let lo = vendor::_mm256_extractf128_si256(self.0.into(), 0).into();
                let hi = vendor::_mm256_extractf128_si256(self.0.into(), 1).into();

                Simd16U32(
                    vendor::_mm256_cvtepu16_epi32(lo).into(),
                    vendor::_mm256_cvtepu16_epi32(hi).into(),
                )
            }
        }

        fn as_i16(self) -> <Self::Mode as SimdMode>::I16 {
            Simd16I16(self.0.into())
        }

        #[inline]
        unsafe fn gather32_ptr(
            base: *const Self::Scalar,
            offset: <Self::Mode as SimdMode>::U32,
            scale: u8,
        ) -> Self {
            // Load 4-byte values
            let data0 = intrin::mm256_i32gather_epi32(
                base as *const _,
                offset.0.into(),
                (scale * 2) as i32,
            );
            let data1 = intrin::mm256_i32gather_epi32(
                base as *const _,
                offset.1.into(),
                (scale * 2) as i32,
            );

            // Throw away the extra 16 MSBs
            Simd16U32(data0.into(), data1.into()).as_u16()
        }
    }

    impl IntPacked for Simd16U16 {
        fn shl_var(self, rhs: <Self::Mode as SimdMode>::U32) -> Self {
            // No `_mm_sllv_epi16` on AVX2, sadly
            self.as_u32().shl_var(rhs).as_u16()
        }
    }
    impl PackedU16 for Simd16U16 {
        fn mul_hi_epu16(self, rhs: Self) -> Self {
            unsafe { Simd16U16(vendor::_mm256_mulhi_epu16(self.0, rhs.0)) }
        }
    }

    #[derive(Debug, Copy, Clone)]
    pub struct Simd16U32(pub simd::u32x8, pub simd::u32x8);

    impl_packed_ops!(Simd16U32, 0, 1);
    impl_int_packed_ops!(Simd16U32, 0, 1);

    impl Simd16U32 {
        #[inline]
        pub(super) fn from_u8(x: Simd16U8) -> Self {
            unsafe {
                let hi = vendor::_mm_shuffle_epi32(x.0.into(), 0b_11_10_11_10);
                Simd16U32(
                    vendor::_mm256_cvtepu8_epi32(x.0.into()).into(),
                    vendor::_mm256_cvtepu8_epi32(hi.into()).into(),
                )
            }
        }
    }

    unsafe impl Packed for Simd16U32 {
        type Scalar = u32;
        type Mode = Simd16Mode;
        const WIDTH: usize = 16;

        #[inline]
        fn table<F: FnMut(usize) -> Self::Scalar>(mut f: F) -> Self {
            Simd16U32(
                simd::u32x8::new(f(0), f(1), f(2), f(3), f(4), f(5), f(6), f(7)),
                simd::u32x8::new(f(8), f(9), f(10), f(11), f(12), f(13), f(14), f(15)),
            )
        }
        #[inline]
        unsafe fn get_unchecked(self, i: usize) -> Self::Scalar {
            if i < 8 {
                self.0.extract_unchecked(i as u32)
            } else {
                self.1.extract_unchecked(i as u32 - 8)
            }
        }
        fn splat(x: Self::Scalar) -> Self {
            Simd16U32(simd::u32x8::splat(x), simd::u32x8::splat(x))
        }

        fn as_u8(self) -> <Self::Mode as SimdMode>::U8 {
            unsafe {
                // Discard upper 24 bits so they don't affect the result of
                // `_mm256_packus_epi32` (which casts `i32` to `u16` with saturation)
                // and  `_mm256_packus_epi16` (which casts `i16` to `u8` with saturation)
                let data0 = self.0 & simd::u32x8::splat(0xff);
                let data1 = self.1 & simd::u32x8::splat(0xff);

                let i16s = vendor::_mm256_packus_epi32(data0.into(), data1.into());
                let i16s = vendor::_mm256_permute4x64_epi64(i16s.into(), 0b_11_01_10_00);

                // Split into two `m128i`s
                let lo = vendor::_mm256_extractf128_si256(i16s.into(), 0).into();
                let hi = vendor::_mm256_extractf128_si256(i16s.into(), 1).into();

                Simd16U8(vendor::_mm_packus_epi16(lo, hi))
            }
        }
        fn as_u16(self) -> <Self::Mode as SimdMode>::U16 {
            unsafe {
                // Discard upper 16 bits so they don't affect the result of
                // `_mm256_packus_epi32` (which casts `i32` to `u16` with saturation)
                let data0 = self.0 & simd::u32x8::splat(0xffff);
                let data1 = self.1 & simd::u32x8::splat(0xffff);

                let i16s = vendor::_mm256_packus_epi32(data0.into(), data1.into());
                let i16s = vendor::_mm256_permute4x64_epi64(i16s.into(), 0b_11_01_10_00);
                Simd16U16(i16s.into())
            }
        }
        fn as_u32(self) -> <Self::Mode as SimdMode>::U32 {
            self
        }

        fn as_i16(self) -> <Self::Mode as SimdMode>::I16 {
            self.as_u16().as_i16()
        }

        #[inline]
        unsafe fn gather32_ptr(
            base: *const Self::Scalar,
            offset: <Self::Mode as SimdMode>::U32,
            scale: u8,
        ) -> Self {
            Simd16U32(
                intrin::mm256_i32gather_epi32(
                    base as *const _,
                    offset.0.into(),
                    (scale * 4) as i32,
                ).into(),
                intrin::mm256_i32gather_epi32(
                    base as *const _,
                    offset.1.into(),
                    (scale * 4) as i32,
                ).into(),
            )
        }
    }

    impl IntPacked for Simd16U32 {
        #[inline]
        fn shl_var(self, rhs: <Self::Mode as SimdMode>::U32) -> Self {
            unsafe {
                Simd16U32(
                    vendor::_mm256_sllv_epi32(self.0.into(), rhs.0.into()).into(),
                    vendor::_mm256_sllv_epi32(self.1.into(), rhs.1.into()).into(),
                )
            }
        }
    }
    impl PackedU32 for Simd16U32 {}

    #[derive(Debug, Copy, Clone)]
    pub struct Simd16I16(pub simd::i16x16);

    impl_packed_ops!(Simd16I16, 0);
    impl_int_packed_ops!(Simd16I16, 0);

    unsafe impl Packed for Simd16I16 {
        type Scalar = i16;
        type Mode = Simd16Mode;
        const WIDTH: usize = 16;

        #[inline]
        fn table<F: FnMut(usize) -> Self::Scalar>(mut f: F) -> Self {
            Simd16I16(simd::i16x16::new(
                f(0),
                f(1),
                f(2),
                f(3),
                f(4),
                f(5),
                f(6),
                f(7),
                f(8),
                f(9),
                f(10),
                f(11),
                f(12),
                f(13),
                f(14),
                f(15),
            ))
        }
        unsafe fn get_unchecked(self, i: usize) -> Self::Scalar {
            self.0.extract_unchecked(i as u32)
        }
        fn splat(x: Self::Scalar) -> Self {
            Simd16I16(simd::i16x16::splat(x))
        }

        fn as_u8(self) -> <Self::Mode as SimdMode>::U8 {
            self.as_u16().as_u8()
        }
        fn as_u16(self) -> <Self::Mode as SimdMode>::U16 {
            Simd16U16(self.0.into())
        }
        fn as_u32(self) -> <Self::Mode as SimdMode>::U32 {
            self.as_u16().as_u32()
        }

        fn as_i16(self) -> <Self::Mode as SimdMode>::I16 {
            self
        }

        unsafe fn gather32_ptr(
            base: *const Self::Scalar,
            offset: <Self::Mode as SimdMode>::U32,
            scale: u8,
        ) -> Self {
            Simd16U16::gather32_ptr(base as *const _, offset, scale).as_i16()
        }
    }

    impl IntPacked for Simd16I16 {}
    impl PackedI16 for Simd16I16 {
        #[inline]
        #[cfg(target_feature = "ssse3")]
        fn mul_hrs_epi16(self, rhs: Self) -> Self {
            unsafe { Simd16I16(vendor::_mm256_mulhrs_epi16(self.0, rhs.0)) }
        }
    }
}

#[cfg(target_feature = "avx2")]
pub use self::avx2::*;

#[cfg(not(target_feature = "avx2"))]
mod generic {
    use super::*;

    #[derive(Debug, Copy, Clone)]
    pub struct Simd16U16(pub simd::u16x8, pub simd::u16x8);

    impl_packed_ops!(Simd16U16, 0, 1);
    impl_int_packed_ops!(Simd16U16, 0, 1);

    unsafe impl Packed for Simd16U16 {
        type Scalar = u16;
        type Mode = Simd16Mode;
        const WIDTH: usize = 16;

        #[inline]
        fn table<F: FnMut(usize) -> Self::Scalar>(mut f: F) -> Self {
            Simd16U16(
                simd::u16x8::new(f(0), f(1), f(2), f(3), f(4), f(5), f(6), f(7)),
                simd::u16x8::new(f(8), f(9), f(10), f(11), f(12), f(13), f(14), f(15)),
            )
        }
        #[inline]
        unsafe fn get_unchecked(self, i: usize) -> Self::Scalar {
            if i < 8 {
                self.0.extract_unchecked(i as u32)
            } else {
                self.1.extract_unchecked(i as u32 - 8)
            }
        }
        fn splat(x: Self::Scalar) -> Self {
            Simd16U16(simd::u16x8::splat(x), simd::u16x8::splat(x))
        }

        fn as_u16(self) -> <Self::Mode as SimdMode>::U16 {
            self
        }

        fn as_i16(self) -> <Self::Mode as SimdMode>::I16 {
            Simd16I16(self.0.into(), self.1.into())
        }
    }

    impl IntPacked for Simd16U16 {}
    impl PackedU16 for Simd16U16 {
        #[inline]
        #[cfg(target_feature = "sse2")]
        fn mul_hi_epu16(self, rhs: Self) -> Self {
            unsafe {
                Simd16U16(
                    vendor::_mm_mulhi_epu16(self.0, rhs.0),
                    vendor::_mm_mulhi_epu16(self.1, rhs.1),
                )
            }
        }
    }

    #[derive(Debug, Copy, Clone)]
    pub struct Simd16U32(
        pub simd::u32x4,
        pub simd::u32x4,
        pub simd::u32x4,
        pub simd::u32x4,
    );

    impl_packed_ops!(Simd16U32, 0, 1, 2, 3);
    impl_int_packed_ops!(Simd16U32, 0, 1, 2, 3);

    unsafe impl Packed for Simd16U32 {
        type Scalar = u32;
        type Mode = Simd16Mode;
        const WIDTH: usize = 16;

        #[inline]
        fn table<F: FnMut(usize) -> Self::Scalar>(mut f: F) -> Self {
            Simd16U32(
                simd::u32x4::new(f(0), f(1), f(2), f(3)),
                simd::u32x4::new(f(4), f(5), f(6), f(7)),
                simd::u32x4::new(f(8), f(9), f(10), f(11)),
                simd::u32x4::new(f(12), f(13), f(14), f(15)),
            )
        }
        #[inline]
        unsafe fn get_unchecked(self, i: usize) -> Self::Scalar {
            if i < 4 {
                self.0.extract_unchecked(i as u32)
            } else if i < 8 {
                self.1.extract_unchecked(i as u32 - 4)
            } else if i < 12 {
                self.2.extract_unchecked(i as u32 - 8)
            } else {
                self.3.extract_unchecked(i as u32 - 12)
            }
        }
        fn splat(x: Self::Scalar) -> Self {
            Simd16U32(
                simd::u32x4::splat(x),
                simd::u32x4::splat(x),
                simd::u32x4::splat(x),
                simd::u32x4::splat(x),
            )
        }

        fn as_u32(self) -> <Self::Mode as SimdMode>::U32 {
            self
        }
    }

    impl IntPacked for Simd16U32 {}
    impl PackedU32 for Simd16U32 {}

    #[derive(Debug, Copy, Clone)]
    pub struct Simd16I16(pub simd::i16x8, pub simd::i16x8);

    impl_packed_ops!(Simd16I16, 0, 1);
    impl_int_packed_ops!(Simd16I16, 0, 1);

    unsafe impl Packed for Simd16I16 {
        type Scalar = i16;
        type Mode = Simd16Mode;
        const WIDTH: usize = 16;

        #[inline]
        fn table<F: FnMut(usize) -> Self::Scalar>(mut f: F) -> Self {
            Simd16I16(
                simd::i16x8::new(f(0), f(1), f(2), f(3), f(4), f(5), f(6), f(7)),
                simd::i16x8::new(f(8), f(9), f(10), f(11), f(12), f(13), f(14), f(15)),
            )
        }
        unsafe fn get_unchecked(self, i: usize) -> Self::Scalar {
            if i < 8 {
                self.0.extract_unchecked(i as u32)
            } else {
                self.1.extract_unchecked(i as u32 - 8)
            }
        }
        fn splat(x: Self::Scalar) -> Self {
            Simd16I16(simd::i16x8::splat(x), simd::i16x8::splat(x))
        }

        fn as_u8(self) -> <Self::Mode as SimdMode>::U8 {
            self.as_u16().as_u8()
        }
        fn as_u16(self) -> <Self::Mode as SimdMode>::U16 {
            Simd16U16(self.0.into(), self.1.into())
        }
        fn as_u32(self) -> <Self::Mode as SimdMode>::U32 {
            self.as_u16().as_u32()
        }

        fn as_i16(self) -> <Self::Mode as SimdMode>::I16 {
            self
        }
    }

    impl IntPacked for Simd16I16 {}
    impl PackedI16 for Simd16I16 {
        #[cfg(target_feature = "sse2")]
        fn mul_hrs_epi16(self, rhs: Self) -> Self {
            #[cfg(target_feature = "ssse3")]
            unsafe fn mulhrs_epi16(x: simd::i16x8, y: simd::i16x8) -> simd::i16x8 {
                vendor::_mm_mulhrs_epi16(x, y)
            }
            #[cfg(not(target_feature = "ssse3"))]
            unsafe fn mulhrs_epi16(x: simd::i16x8, y: simd::i16x8) -> simd::i16x8 {
                let lo = vendor::_mm_mullo_epi16(x, y);
                let hi = vendor::_mm_mulhi_epi16(x, y);
                let lo_14 = vendor::_mm_srli_epi16(vendor::_mm_slli_epi16(lo, 1), 15);
                let lo_15 = vendor::_mm_srli_epi16(lo, 15);
                vendor::_mm_slli_epi16(hi, 1) + lo_15 + lo_14
            }
            unsafe { Simd16I16(mulhrs_epi16(self.0, rhs.0), mulhrs_epi16(self.1, rhs.1)) }
        }
    }
}

#[cfg(not(target_feature = "avx2"))]
pub use self::generic::*;
