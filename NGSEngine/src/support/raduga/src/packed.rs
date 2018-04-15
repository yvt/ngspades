//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use num_traits::AsPrimitive;
use std::ops;

/// A packed vector type. Performs operations on the individual elements.
pub unsafe trait Packed:
    Copy
    + Sized
    + ::std::fmt::Debug
    + ops::Add<Output = Self>
    + ops::Sub<Output = Self>
    + ops::AddAssign
    + ops::SubAssign
{
    /// The type of the element.
    type Scalar: Copy
        + AsPrimitive<u8>
        + AsPrimitive<u16>
        + AsPrimitive<u32>
        + AsPrimitive<i16>
        + Default;
    type Mode: SimdMode;

    /// The number of elements in this vector.
    const WIDTH: usize;

    /// Construct a vector by generating its elements by a given function that
    /// takes an element index as its input.
    fn table<F: FnMut(usize) -> Self::Scalar>(f: F) -> Self;
    /// Extract an element of a vector.
    unsafe fn get_unchecked(self, i: usize) -> Self::Scalar;
    /// Extract an element of a vector.
    #[inline]
    fn get(self, i: usize) -> Self::Scalar {
        assert!(i < Self::WIDTH);
        unsafe { self.get_unchecked(i) }
    }

    /// Apply a function on each element and construct a new vector of a
    /// different type.
    #[inline]
    fn map_to<T: Packed, F: FnMut(Self::Scalar) -> T::Scalar>(self, mut f: F) -> T {
        unsafe { T::table(|i| f(self.get_unchecked(i))) }
    }
    /// Apply a function on each element and construct a new vector of the same
    /// type.
    #[inline]
    fn map<F: FnMut(Self::Scalar) -> Self::Scalar>(self, f: F) -> Self {
        self.map_to(f)
    }

    #[inline]
    fn splat(x: Self::Scalar) -> Self {
        Self::table(|_| x)
    }

    /// Cast each element to `u8` and construct a new vector.
    #[inline]
    fn as_u8(self) -> <Self::Mode as SimdMode>::U8 {
        self.map_to(|x| x.as_())
    }
    /// Cast each element to `u16` and construct a new vector.
    #[inline]
    fn as_u16(self) -> <Self::Mode as SimdMode>::U16 {
        self.map_to(|x| x.as_())
    }
    /// Cast each element to `u32` and construct a new vector.
    #[inline]
    fn as_u32(self) -> <Self::Mode as SimdMode>::U32 {
        self.map_to(|x| x.as_())
    }

    /// Cast each element to `i16` and construct a new vector.
    #[inline]
    fn as_i16(self) -> <Self::Mode as SimdMode>::I16 {
        self.map_to(|x| x.as_())
    }

    /// Load `WIDTH` values from non-contiguous memory locations.
    ///
    /// The load address is computed as `base.offset(offset * scale)`.
    ///
    /// The load may be implemented by 4-byte sized loads. This means that if
    /// the scalar type is `u8`, 3 bytes following the specified location may
    /// be actually accessed.
    #[inline]
    unsafe fn gather32_ptr(
        base: *const Self::Scalar,
        offset: <Self::Mode as SimdMode>::U32,
        scale: u8,
    ) -> Self {
        offset.map_to(|x| *base.offset(x as isize * scale as isize))
    }

    /// Load `WIDTH` values from non-contiguous memory locations inside a slice.
    /// Bounds check is only performed on debug builds.
    ///
    /// The load index is computed as `slice[offset * scale]`.
    #[inline]
    unsafe fn gather32_unchecked(
        slice: &[Self::Scalar],
        offset: <Self::Mode as SimdMode>::U32,
        scale: u8,
    ) -> Self {
        if cfg!(debug_assertions) {
            use std::mem::size_of;
            // "The load may be implemented by 4-byte sized loads."
            let overshoot = (size_of::<u32>() / size_of::<Self::Scalar>()).saturating_sub(1);

            for i in 0..Self::WIDTH {
                let index = offset.get_unchecked(i) as usize * scale as usize;
                assert!(index + overshoot < slice.len());
            }
        }
        Self::gather32_ptr(slice.as_ptr(), offset, scale)
    }
}

pub trait IntPacked:
    Packed
    + ops::Shr<u32, Output = Self>
    + ops::Shl<u32, Output = Self>
    + ops::ShrAssign<u32>
    + ops::ShlAssign<u32>
{
    /// Perform a left shift by a variable amount on each component.
    #[inline]
    fn shl_var(self, rhs: <Self::Mode as SimdMode>::U32) -> Self
    where
        Self::Scalar: ops::Shl<u32, Output = Self::Scalar>,
    {
        Self::table(|i| unsafe { self.get_unchecked(i) << rhs.get_unchecked(i) })
    }

    // TODO: Wrapping add/sub
}

pub trait PackedU8: Packed<Scalar = u8> + IntPacked {}
pub trait PackedU16: Packed<Scalar = u16> + IntPacked {
    /// Multiply and extract the 16 most significant bits of the 32-bit result.
    #[inline]
    fn mul_hi_epu16(self, rhs: Self) -> Self {
        Self::table(|i| unsafe {
            ((self.get_unchecked(i) as u32 * rhs.get_unchecked(i) as u32) >> 16) as u16
        })
    }
}
pub trait PackedU32: Packed<Scalar = u32> + IntPacked {}

pub trait PackedI16: Packed<Scalar = i16> + IntPacked {
    /// Multiply and shift right by 15 bits with rounding.
    ///
    /// This operation can be explained as following: Treat operands as signed
    /// 1.15 fixed point numbers. Multiply them together with correct
    /// rounding.
    #[inline]
    fn mul_hrs_epi16(self, rhs: Self) -> Self {
        Self::table(|i| unsafe {
            ((self.get_unchecked(i) as i32 * rhs.get_unchecked(i) as i32 + 16384) >> 15) as i16
        })
    }
}

/// A set of `Packed` types with the same SIMD operation mode.
pub trait SimdMode: Copy + ::std::fmt::Debug + Sized {
    type U8: Packed<Mode = Self> + PackedU8;
    type U16: Packed<Mode = Self> + PackedU16;
    type U32: Packed<Mode = Self> + PackedU32;
    type I16: Packed<Mode = Self> + PackedI16;
}

#[derive(Debug, Copy, Clone)]
pub struct ScalarMode;

impl SimdMode for ScalarMode {
    type U8 = u8;
    type U16 = u16;
    type U32 = u32;
    type I16 = i16;
}

macro_rules! impl_scalar {
    ($type:ty) => {
        unsafe impl Packed for $type {
            type Scalar = Self;
            type Mode = ScalarMode;
            const WIDTH: usize = 1;

            #[inline]
            fn table<F: FnMut(usize) -> Self::Scalar>(mut f: F) -> Self {
                f(0)
            }
            #[inline]
            unsafe fn get_unchecked(self, _: usize) -> Self::Scalar {
                self
            }
        }
    };
}

impl_scalar!(u8);
impl IntPacked for u8 {}
impl PackedU8 for u8 {}

impl_scalar!(u16);
impl IntPacked for u16 {}
impl PackedU16 for u16 {}

impl_scalar!(u32);
impl IntPacked for u32 {}
impl PackedU32 for u32 {}

impl_scalar!(i16);
impl IntPacked for i16 {}
impl PackedI16 for i16 {}

/// Implements basic operations required by `Packed` by applying them to the
/// type's fields.
macro_rules! impl_packed_ops {
    ($type:tt, $($field:tt),*) => {
        impl ::std::ops::Add for $type {
            type Output = Self;
            #[inline]
            fn add(self, rhs: Self) -> Self {
                $type($(self.$field + rhs.$field),*)
            }
        }
        impl ::std::ops::Sub for $type {
            type Output = Self;
            #[inline]
            fn sub(self, rhs: Self) -> Self {
                $type($(self.$field - rhs.$field),*)
            }
        }
        impl ::std::ops::AddAssign for $type {
            #[inline]
            fn add_assign(&mut self, rhs: Self) {
                *self = *self + rhs;
            }
        }
        impl ::std::ops::SubAssign for $type {
            #[inline]
            fn sub_assign(&mut self, rhs: Self) {
                *self = *self - rhs;
            }
        }
    }
}

/// Implements basic operations required by `IntPacked` by applying them to
/// the type's fields.
macro_rules! impl_int_packed_ops {
    ($type:tt, $($field:tt),*) => {
        impl ::std::ops::Shr<u32> for $type {
            type Output = Self;
            #[inline]
            fn shr(self, rhs: u32) -> Self {
                $type($(self.$field >> rhs),*)
            }
        }
        impl ::std::ops::Shl<u32> for $type {
            type Output = Self;
            #[inline]
            fn shl(self, rhs: u32) -> Self {
                $type($(self.$field << rhs),*)
            }
        }
        impl ::std::ops::ShlAssign<u32> for $type {
            #[inline]
            fn shl_assign(&mut self, rhs: u32) {
                *self = *self << rhs;
            }
        }
        impl ::std::ops::ShrAssign<u32> for $type {
            #[inline]
            fn shr_assign(&mut self, rhs: u32) {
                *self = *self >> rhs;
            }
        }
    }
}
