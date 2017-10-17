//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;
use cgmath::Vector2;
use context::{RefPropertyAccessor, RoPropertyAccessor};

/// Owned raster image data.
#[derive(Debug, Clone)]
pub struct ImageData {
    pixels: Vec<u32>,
    size: Vector2<usize>,
    format: ImageFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// Represents a pixel format with a 8-bit red/green/blue/alpha channels in
    /// the sRGB encoding and in BGRA order.
    SrgbBgra8,
}

impl ImageData {
    /// Construct an `ImageData`.
    pub fn new(size: Vector2<usize>, format: ImageFormat) -> Self {
        let num_pixels = size.x.checked_mul(size.y).expect("size overflow");
        Self {
            pixels: vec![0; num_pixels],
            size,
            format,
        }
    }

    pub fn size(&self) -> Vector2<usize> {
        self.size
    }

    pub fn format(&self) -> ImageFormat {
        self.format
    }

    pub fn pixels_u32(&self) -> &[u32] {
        self.pixels.as_slice()
    }

    pub fn pixels_u32_mut(&mut self) -> &mut [u32] {
        self.pixels.as_mut_slice()
    }
}

/// Reference to an immutable raster image.
#[derive(Debug, Clone)]
pub struct ImageRef(Arc<ImageData>);

impl ImageRef {
    pub fn new_immutable(data: ImageData) -> Self {
        ImageRef(Arc::new(data))
    }

    pub fn image_data<'a>(&'a self) -> impl RoPropertyAccessor<ImageData> + 'a {
        RefPropertyAccessor::new(&*self.0)
    }
}
