//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Add;

/// An iterator producing a sequence of vectors defined as
/// `x_i = start + step * i`.
#[derive(Debug)]
pub struct LinePoints<T> {
    pub cur: T,
    pub step: T,
}

impl<T> LinePoints<T> {
    pub fn new(start: T, step: T) -> Self {
        Self { cur: start, step }
    }

    /// Apply a function to each of `cur` and `step`.
    pub fn map_linear<U>(self, mut f: impl FnMut(T) -> U) -> LinePoints<U> {
        LinePoints {
            cur: f(self.cur),
            step: f(self.step),
        }
    }
}

impl<T> Iterator for LinePoints<T>
where
    T: Add<Output = T> + Clone,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.cur.clone();
        self.cur = self.cur.clone() + self.step.clone();
        Some(ret)
    }
}

