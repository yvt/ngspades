//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Vector2;

/// A depth image on memory.
pub struct DepthImage {
    pub(crate) image: Vec<f32>,
    size: Vector2<usize>,
}

impl DepthImage {
    /// Construct a `DepthImage`.
    pub fn new(size: Vector2<usize>) -> Self {
        Self {
            image: vec![0.0; size.x.checked_mul(size.y).unwrap()],
            size,
        }
    }

    /// Return the dimensions of the depth image.
    pub fn size(&self) -> Vector2<usize> {
        self.size
    }

    /// Return the raw representation of the depth image.
    pub fn pixels(&self) -> &[f32] {
        &self.image[..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create() {
        DepthImage::new(Vector2::new(256, 256));
    }
}
