//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::fmt::Debug;
use std::ops::{Add, Sub, Mul, Div, AddAssign, SubAssign, MulAssign, DivAssign, Neg};
use std::default::Default;
use cgmath::num_traits::{Zero, One};
pub use cgmath::num_traits::Float;
pub use cgmath::BaseNum;

/// Frequency-dependent quantity, defined for each frequency band.
pub trait BaseFdQuant: Debug + Clone + Copy + Sized + Zero + One + Default
where
    Self: Add<Output = Self>,
    Self: Sub<Output = Self>,
    Self: Mul<Output = Self>,
    Self: Mul<<Self as BaseFdQuant>::Scalar, Output = Self>,
    Self: Div<<Self as BaseFdQuant>::Scalar, Output = Self>,
    Self: AddAssign,
    Self: SubAssign,
    Self: MulAssign,
    Self: MulAssign<<Self as BaseFdQuant>::Scalar>,
    Self: DivAssign<<Self as BaseFdQuant>::Scalar>,
    Self: Neg<Output = Self>,
{
    type Scalar: BaseNum + Float;

    /// The number of frequency bands, or `None` if it depends on the value.
    const BANDS: Option<usize> = None;

    fn exp(self) -> Self;
}

impl<T: BaseNum + Float + Default> BaseFdQuant for T {
    type Scalar = Self;

    fn exp(self) -> Self {
        Float::exp(self)
    }
}

/// Frequency-dependent quantity, defined for each frequency band.
#[derive(Debug, Clone, Copy)]
pub struct FdQuant<T>(T);

impl<T> FdQuant<T> {
    pub fn new(x: T) -> Self {
        FdQuant(x)
    }

    pub fn get_ref(&self) -> &T {
        &self.0
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: AsRef<U>, U: ?Sized> AsRef<U> for FdQuant<T> {
    fn as_ref(&self) -> &U {
        self.0.as_ref()
    }
}

impl<T: AsMut<U>, U: ?Sized> AsMut<U> for FdQuant<T> {
    fn as_mut(&mut self) -> &mut U {
        self.0.as_mut()
    }
}

macro_rules! fdq_impl {
    ($num:expr; ($($idx:expr),*)) => (
        impl<T: BaseNum + Float> BaseFdQuant for FdQuant<[T; $num]> {
            type Scalar = T;

            const BANDS: Option<usize> = Some($num);

            fn exp(self) -> Self {
                FdQuant([$(self.0[$idx].exp()),*])
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> Zero for FdQuant<[T; $num]> {
            fn zero() -> Self {
                FdQuant([Zero::zero(); $num])
            }

            fn is_zero(&self) -> bool {
                self.0.iter().all(|x| x.is_zero())
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> One for FdQuant<[T; $num]> {
            fn one() -> Self {
                FdQuant([One::one(); $num])
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> Default for FdQuant<[T; $num]> {
            fn default() -> Self {
                Self::zero()
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> Neg for FdQuant<[T; $num]> {
            type Output = Self;
            fn neg(self) -> Self::Output {
                FdQuant([$(-self.0[$idx]),*])
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> Add for FdQuant<[T; $num]> {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                FdQuant([$(self.0[$idx] + rhs.0[$idx]),*])
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> Sub for FdQuant<[T; $num]> {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self::Output {
                FdQuant([$(self.0[$idx] - rhs.0[$idx]),*])
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> Mul for FdQuant<[T; $num]> {
            type Output = Self;
            fn mul(self, rhs: Self) -> Self::Output {
                FdQuant([$(self.0[$idx] * rhs.0[$idx]),*])
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> Mul<T> for FdQuant<[T; $num]> {
            type Output = Self;
            fn mul(self, rhs: T) -> Self::Output {
                FdQuant([$(self.0[$idx] * rhs),*])
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> Div<T> for FdQuant<[T; $num]> {
            type Output = Self;
            fn div(self, rhs: T) -> Self::Output {
                FdQuant([$(self.0[$idx] / rhs),*])
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> AddAssign for FdQuant<[T; $num]> {
            fn add_assign(&mut self, rhs: Self) {
                $(self.0[$idx] += rhs.0[$idx];)*
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> SubAssign for FdQuant<[T; $num]> {
            fn sub_assign(&mut self, rhs: Self) {
                $(self.0[$idx] -= rhs.0[$idx];)*
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> MulAssign for FdQuant<[T; $num]> {
            fn mul_assign(&mut self, rhs: Self) {
                $(self.0[$idx] *= rhs.0[$idx];)*
            }
        }
        #[doc(hidden)]
        impl<T: BaseNum + Float> MulAssign<T> for FdQuant<[T; $num]> {
            fn mul_assign(&mut self, rhs: T) {
                $(self.0[$idx] *= rhs;)*
            }
        }

        #[doc(hidden)]
        impl<T: BaseNum + Float> DivAssign<T> for FdQuant<[T; $num]> {
            fn div_assign(&mut self, rhs: T) {
                $(self.0[$idx] /= rhs;)*
            }
        }
    )
}

fdq_impl! { 1; (0) }
fdq_impl! { 2; (0, 1) }
fdq_impl! { 3; (0, 1, 2) }
fdq_impl! { 4; (0, 1, 2, 3) }
fdq_impl! { 5; (0, 1, 2, 3, 4) }
fdq_impl! { 6; (0, 1, 2, 3, 4, 5) }
fdq_impl! { 7; (0, 1, 2, 3, 4, 5, 6) }
fdq_impl! { 8; (0, 1, 2, 3, 4, 5, 6, 7) }
fdq_impl! { 12; (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11) }
fdq_impl! { 16; (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15) }
