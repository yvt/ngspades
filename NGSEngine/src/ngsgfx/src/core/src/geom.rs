//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

use cgmath::Vector2;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Rect2D<T> {
    pub min: Vector2<T>,
    pub max: Vector2<T>,
}

impl<T> Rect2D<T> {
    pub fn new(min: Vector2<T>, max: Vector2<T>) -> Self {
        Self { min: min, max: max }
    }
}
