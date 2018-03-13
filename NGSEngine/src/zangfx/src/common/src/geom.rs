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
