//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a trait for enumerating all possible values of a type.
//!
//! # Examples
//!
//!     extern crate itervalues;
//!     #[macro_use]
//!     extern crate itervalues_derive;
//!
//!     use itervalues::IterValues;
//!
//!     # fn main() {
//!     let all_bools: Vec<_> = <bool>::iter_values().collect();
//!     assert_eq!(all_bools.as_slice(), &[false, true]);
//!
//!     #[derive(IterValues, Copy, Clone, PartialEq, Eq, Debug)]
//!     enum Test { X(bool), Y }
//!
//!     let all_values: Vec<_> = Test::iter_values().collect();
//!     assert_eq!(
//!         all_values.as_slice(),
//!         &[Test::X(false), Test::X(true), Test::Y]
//!     );
//!
//!     # }
//!
use std::iter::{self, ExactSizeIterator, Iterator};
use std::slice;

/// Returns an iterator that enumerates all possible values of a type.
pub trait IterValues: Sized {
    type Iterator: Iterator<Item = Self>;

    /// Retrieve an iterator that enumerates all possible values of this type.
    fn iter_values() -> Self::Iterator;
}

impl IterValues for () {
    type Iterator = iter::Cloned<slice::Iter<'static, Self>>;

    fn iter_values() -> Self::Iterator {
        [()].into_iter().cloned()
    }
}

impl IterValues for bool {
    type Iterator = iter::Cloned<slice::Iter<'static, Self>>;

    fn iter_values() -> Self::Iterator {
        [false, true].into_iter().cloned()
    }
}

impl<T: IterValues> IterValues for Option<T> {
    type Iterator = OptionIterValues<T>;

    fn iter_values() -> Self::Iterator {
        OptionIterValues(None)
    }
}

pub struct OptionIterValues<T: IterValues>(Option<T::Iterator>);

impl<T: IterValues> Iterator for OptionIterValues<T> {
    type Item = Option<T>;

    fn next(&mut self) -> Option<Option<T>> {
        if self.0.is_none() {
            self.0 = Some(T::iter_values());
            Some(None)
        } else {
            self.0.as_mut().unwrap().next().map(Some)
        }
    }
}

impl<T1: IterValues> IterValues for (T1,) {
    type Iterator = WrapTuple<T1::Iterator>;

    fn iter_values() -> Self::Iterator {
        WrapTuple(T1::iter_values())
    }
}

/// An iterator that wraps the inner iterator's value with `(x,)`.
#[derive(Debug, Copy, Clone)]
pub struct WrapTuple<T>(T);

impl<T: Iterator> Iterator for WrapTuple<T> {
    type Item = (T::Item,);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|v| (v,))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<T: Iterator + ExactSizeIterator> ExactSizeIterator for WrapTuple<T> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<T1: IterValues + Clone, T2: IterValues> IterValues for (T1, T2) {
    type Iterator = PairIterValues<T1, T2>;

    fn iter_values() -> Self::Iterator {
        let mut iter1 = T1::iter_values();
        let value1 = iter1.next();
        let iter2 = T2::iter_values();
        PairIterValues {
            count: match (iter1.size_hint(), iter2.size_hint()) {
                ((_, Some(len1)), (_, Some(len2))) => len1.checked_mul(len2),
                _ => None,
            },
            iter1,
            value1,
            iter2,
        }
    }
}

pub struct PairIterValues<T1: IterValues + Clone, T2: IterValues> {
    iter1: T1::Iterator,
    value1: Option<T1>,
    iter2: T2::Iterator,
    count: Option<usize>,
}

impl<T1: IterValues + Clone, T2: IterValues> Iterator for PairIterValues<T1, T2> {
    type Item = (T1, T2);

    fn next(&mut self) -> Option<(T1, T2)> {
        if let Some(value1) = self.value1.clone() {
            if let Some(value2) = self.iter2.next() {
                return Some((value1, value2));
            }

            self.value1 = self.iter1.next();
            if let Some(value1) = self.value1.clone() {
                self.iter2 = T2::iter_values();
                if let Some(value2) = self.iter2.next() {
                    return Some((value1, value2));
                } else {
                    self.value1 = None;
                }
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.count)
    }
}

impl<T1: IterValues + Clone, T2: IterValues + Clone, T3: IterValues> IterValues for (T1, T2, T3) {
    type Iterator = Flatten3<<(T1, (T2, T3)) as IterValues>::Iterator>;

    fn iter_values() -> Self::Iterator {
        Flatten3(<(T1, (T2, T3))>::iter_values())
    }
}

/// An iterator that expands the inner iterator's value `(v1, (v2, v3))` to
/// `(v1, v2, v3)`.
#[derive(Debug, Copy, Clone)]
pub struct Flatten3<T>(T);

impl<T, T1, T2, T3> Iterator for Flatten3<T>
where
    T: Iterator<Item = (T1, (T2, T3))>,
{
    type Item = (T1, T2, T3);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(v1, (v2, v3))| (v1, v2, v3))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<T, T1, T2, T3> ExactSizeIterator for Flatten3<T>
where
    T: Iterator<Item = (T1, (T2, T3))> + ExactSizeIterator,
{
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<
    T1: IterValues + Clone,
    T2: IterValues + Clone,
    T3: IterValues + Clone,
    T4: IterValues,
> IterValues for (T1, T2, T3, T4)
{
    type Iterator = Flatten4<<(T1, (T2, (T3, T4))) as IterValues>::Iterator>;

    fn iter_values() -> Self::Iterator {
        Flatten4(<(T1, (T2, (T3, T4)))>::iter_values())
    }
}

/// An iterator that expands the inner iterator's value `(v1, (v2, (v3, v4)))` to
/// `(v1, v2, v3, v4)`.
#[derive(Debug, Copy, Clone)]
pub struct Flatten4<T>(T);

impl<T, T1, T2, T3, T4> Iterator for Flatten4<T>
where
    T: Iterator<Item = (T1, (T2, (T3, T4)))>,
{
    type Item = (T1, T2, T3, T4);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(v1, (v2, (v3, v4)))| (v1, v2, v3, v4))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<T, T1, T2, T3, T4> ExactSizeIterator for Flatten4<T>
where
    T: Iterator<Item = (T1, (T2, (T3, T4)))> + ExactSizeIterator,
{
    fn len(&self) -> usize {
        self.0.len()
    }
}
