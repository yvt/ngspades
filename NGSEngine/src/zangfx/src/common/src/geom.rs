//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Rect2D<T> {
    pub min: [T; 2],
    pub max: [T; 2],
}

impl<T> Rect2D<T> {
    pub fn new<S: Into<[T; 2]>>(min: S, max: S) -> Self {
        Self {
            min: min.into(),
            max: max.into(),
        }
    }
}

impl Rect2D<u32> {
    /// Return `Rect2D::new([0; 2], [<u32>::max_value(); 2])`.
    pub fn all() -> Self {
        Rect2D::new([0; 2], [<u32>::max_value(); 2])
    }
}

/// Take a value and convert it to `Self`, filling the missing places with a
/// given pad value.
pub trait FromWithPad<T, P>: Sized {
    /// Take a value and convert it to `Self`, filling the missing places with a
    /// given pad value.
    fn from_with_pad(x: T, pad: P) -> Self;
}

/// Consume `self` and construct a value, filling the missing places with a
/// given pad value.
pub trait IntoWithPad<T, P>: Sized {
    /// Consume `self` and construct a value, filling the missing places with a
    /// given pad value.
    fn into_with_pad(self, pad: P) -> T;
}

impl<T, U, P> IntoWithPad<U, P> for T
where
    U: FromWithPad<T, P>,
{
    fn into_with_pad(self, pad: P) -> U {
        U::from_with_pad(self, pad)
    }
}

impl<'a, T: Copy + 'static> FromWithPad<&'a [T], T> for [T; 2] {
    /// Make a three-element `T` array from a given slice, filling the missing
    /// elements using a provided pad value.
    ///
    /// Due to performance optimization, this might return a spurious value for
    /// extremely large slices.
    ///
    /// # Examples
    ///
    ///     # use zangfx_common::*;
    ///     assert_eq!(<[u32; 2]>::from_with_pad(&[], 5), [5, 5]);
    ///     assert_eq!(<[u32; 2]>::from_with_pad(&[1], 5), [1, 5]);
    ///     assert_eq!(<[u32; 2]>::from_with_pad(&[1, 2], 5), [1, 2]);
    ///     assert_eq!(<[u32; 2]>::from_with_pad(&[1, 2, 3], 5), [1, 2]);
    ///     assert_eq!(<[u32; 2]>::from_with_pad(&[1, 2, 3, 4], 5), [1, 2]);
    ///
    #[inline]
    fn from_with_pad(x: &'a [T], pad: T) -> Self {
        [
            x.get(0).cloned().unwrap_or(pad),
            x.get(1).cloned().unwrap_or(pad),
        ]
    }
}

impl<'a, T: Copy + 'static> FromWithPad<&'a [T], T> for [T; 3] {
    /// Make a three-element `T` array from a given slice, filling the missing
    /// elements using a provided pad value.
    ///
    /// Due to performance optimization, this might return a spurious value for
    /// extremely large slices.
    ///
    /// # Examples
    ///
    ///     # use zangfx_common::*;
    ///     assert_eq!(<[u32; 3]>::from_with_pad(&[], 5), [5, 5, 5]);
    ///     assert_eq!(<[u32; 3]>::from_with_pad(&[1], 5), [1, 5, 5]);
    ///     assert_eq!(<[u32; 3]>::from_with_pad(&[1, 2], 5), [1, 2, 5]);
    ///     assert_eq!(<[u32; 3]>::from_with_pad(&[1, 2, 3], 5), [1, 2, 3]);
    ///     assert_eq!(<[u32; 3]>::from_with_pad(&[1, 2, 3, 4], 5), [1, 2, 3]);
    ///
    #[inline]
    fn from_with_pad(x: &'a [T], pad: T) -> Self {
        [
            x.get(0).cloned().unwrap_or(pad),
            x.get(1).cloned().unwrap_or(pad),
            x.get(2).cloned().unwrap_or(pad),
        ]
    }
}
