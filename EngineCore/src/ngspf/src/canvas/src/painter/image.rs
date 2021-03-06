//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{prelude::*, Vector2};
use rgb::RGBA;
use std::ops::Range;

use crate::painter::{text::rasterize_text_layout, Painter};
use crate::text::{FontConfig, TextLayout};
use crate::{Affine2, ImageData, ImageFormat};

#[derive(Debug)]
struct ImagePainter<T> {
    port: T,

    /// The current transformation.
    transform: Affine2<f64>,

    transform_stack: Vec<Affine2<f64>>,

    /// The current fill color (alpha-premul).
    fill_color: RGBA<f32>,
}

pub fn new_painter_for_image_data<'a>(
    image_data: &'a mut ImageData,
) -> Box<Painter + Sync + Send + 'a> {
    match image_data.format() {
        ImageFormat::SrgbRgba8 => Box::new(ImagePainter::new(SrgbRgba8RasterPort(image_data))),
        ImageFormat::SrgbRgba8Premul => {
            Box::new(ImagePainter::new(SrgbRgba8PremulRasterPort(image_data)))
        }
    }
}

impl<T> ImagePainter<T> {
    fn new(port: T) -> Self {
        Self {
            port,
            transform: One::one(),
            transform_stack: Vec::new(),
            fill_color: RGBA::new(0.0, 0.0, 0.0, 1.0),
        }
    }
}

impl<T: RasterPort> Painter for ImagePainter<T> {
    fn save(&mut self) {
        self.transform_stack.push(self.transform);
    }

    fn restore(&mut self) {
        self.transform = self.transform_stack.pop().expect("stack is empty");
    }

    fn translate(&mut self, x: Vector2<f64>) {
        self.transform *= Affine2::from_translation(x);
    }

    fn nonuniform_scale(&mut self, x: f64, y: f64) {
        self.transform *= Affine2::from_nonuniform_scale(x, y);
    }

    fn set_fill_color(&mut self, color: RGBA<f32>) {
        self.fill_color = color;
    }

    fn fill_text_layout(&mut self, layout: &TextLayout, config: &FontConfig) {
        rasterize_text_layout(
            &mut self.port,
            self.transform,
            self.fill_color,
            layout,
            config,
        )
    }
}

use super::blend;

pub(crate) trait RasterPort: crate::Debug {
    fn size(&self) -> Vector2<usize>;

    /// The color type used by the intermediate calculation.
    type FastColor: Copy;

    /// `color` is not alpha pre-multiplied
    fn to_fast_color(&self, color: RGBA<f32>) -> Self::FastColor;

    /// Fill a span with a given color. The alpha value is multiplied by
    /// the coverage value.
    fn fill_span_cov(
        &mut self,
        y: usize,
        x_range: Range<usize>,
        color: Self::FastColor,
        coverage: u8,
    );
}

#[derive(Debug)]
struct SrgbRgba8RasterPort<'a>(&'a mut ImageData);

impl<'a> RasterPort for SrgbRgba8RasterPort<'a> {
    fn size(&self) -> Vector2<usize> {
        self.0.size()
    }

    type FastColor = blend::Srgb8InternalColor;

    fn to_fast_color(&self, color: RGBA<f32>) -> Self::FastColor {
        blend::srgb8_color_to_internal(color)
    }

    fn fill_span_cov(
        &mut self,
        y: usize,
        x_range: Range<usize>,
        color: Self::FastColor,
        coverage: u8,
    ) {
        use raduga::prelude::*;

        let stride = self.0.size().x;
        let offset_y = y * stride;

        assert!(y < self.0.size().y);
        assert!(x_range.end <= self.0.size().x);
        if x_range.start >= x_range.end {
            return;
        }

        let src_int = blend::srgb8_internal_mask(color, coverage);

        let pixels_u32 = self.0.pixels_u32_mut();
        let pixels_u8 = pixels_u32.as_mut_ptr() as *mut u8;

        use std::slice::from_raw_parts_mut;
        let span_start = (offset_y + x_range.start) * 4;
        let span_len = x_range.len() * 4;
        let span_pixels_u8 =
            unsafe { from_raw_parts_mut(pixels_u8.wrapping_offset(span_start as isize), span_len) };

        struct Kernel {
            src_int: blend::Srgb8InternalColor,
        }
        impl MapU8x4InplaceKernel for Kernel {
            #[inline]
            fn apply<M: SimdMode>(&self, x: [M::U8; 4]) -> [M::U8; 4] {
                blend::srgb8_alpha_over::<M>(self.src_int, x)
            }
        }

        Kernel { src_int }.dispatch(span_pixels_u8);
    }
}

#[derive(Debug)]
struct SrgbRgba8PremulRasterPort<'a>(&'a mut ImageData);

impl<'a> RasterPort for SrgbRgba8PremulRasterPort<'a> {
    fn size(&self) -> Vector2<usize> {
        self.0.size()
    }

    type FastColor = blend::Srgb8PremulInternalColor;

    fn to_fast_color(&self, color: RGBA<f32>) -> Self::FastColor {
        blend::srgb8_premul_color_to_internal(color)
    }

    fn fill_span_cov(
        &mut self,
        y: usize,
        x_range: Range<usize>,
        color: Self::FastColor,
        coverage: u8,
    ) {
        use raduga::prelude::*;

        let stride = self.0.size().x;
        let offset_y = y * stride;

        assert!(y < self.0.size().y);
        assert!(x_range.end <= self.0.size().x);
        if x_range.start >= x_range.end {
            return;
        }

        let src_int = blend::srgb8_internal_mask(color, coverage);

        let pixels_u32 = self.0.pixels_u32_mut();
        let pixels_u8 = pixels_u32.as_mut_ptr() as *mut u8;

        use std::slice::from_raw_parts_mut;
        let span_start = (offset_y + x_range.start) * 4;
        let span_len = x_range.len() * 4;
        let span_pixels_u8 =
            unsafe { from_raw_parts_mut(pixels_u8.wrapping_offset(span_start as isize), span_len) };

        struct Kernel {
            src_int: blend::Srgb8PremulInternalColor,
        }
        impl MapU8x4InplaceKernel for Kernel {
            #[inline]
            fn apply<M: SimdMode>(&self, x: [M::U8; 4]) -> [M::U8; 4] {
                blend::srgb8_premul_alpha_over::<M>(self.src_int, x)
            }
        }

        Kernel { src_int }.dispatch(span_pixels_u8);
    }
}
