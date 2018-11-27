//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::IndexMut;

/// Iterator-like type that produces a sequence of mutable references.
///
/// `IteratorMut<Item = T>` is different from `Iterator<Item = &'a mut T>`
/// because `IteratorMut::next` returns a mutable reference whose lifetime is
/// bound to `self`.
pub trait IteratorMut {
    type Item: ?Sized;

    fn next(&mut self) -> Option<&mut Self::Item>;

    /// Create an `Iterator` which `clone`s all of its elements.
    ///
    /// This is useful when you have to convert an `IteratorMut` into a
    /// normal iterator.
    fn cloned(self) -> Cloned<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        Cloned(self)
    }
}

pub struct Cloned<I>(I);

impl<I> Iterator for Cloned<I>
where
    I: IteratorMut,
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|x| x.clone())
    }
}

/// An extension trait for `Iterator` involving a conversion to `IteratorMut`.
pub trait IteratorToIteratorMutExt: Iterator {
    /// Gather elements from a given collection as a sequence of mutable
    /// references using the indices produced by this iterator.
    fn gather_mut<'a, C>(self, collection: &'a mut C) -> GatherMut<'a, C, Self>
    where
        C: IndexMut<Self::Item>,
        Self: Sized,
    {
        GatherMut {
            iter: self,
            collection,
        }
    }
}

impl<T: ?Sized + Iterator> IteratorToIteratorMutExt for T {}

/// An `IteratorMut` that produce a sequence of mutable references to elements
/// of a collection using indices provided by a normal iterator.
pub struct GatherMut<'a, C, I> {
    iter: I,
    collection: &'a mut C,
}

impl<'a, C, I> IteratorMut for GatherMut<'a, C, I>
where
    I: Iterator,
    C: IndexMut<I::Item>,
{
    type Item = C::Output;

    fn next(&mut self) -> Option<&mut Self::Item> {
        if let Some(i) = self.iter.next() {
            Some(&mut self.collection[i])
        } else {
            None
        }
    }
}
