//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{prelude::*, Vector2};
use rgb::RGBA;
use std::ops::Range;

use painter::{text::rasterize_text_layout, Painter};
use text::{FontConfig, TextLayout};
use {Affine2, ImageData};

#[derive(Debug)]
pub(super) struct ImagePainter<'a> {
    image: &'a mut ImageData,

    /// The current transformation.
    transform: Affine2<f64>,

    transform_stack: Vec<Affine2<f64>>,

    /// The current fill color (alpha-premul).
    fill_color: RGBA<f32>,
}

pub fn new_painter_for_image_data<'a>(
    image_data: &'a mut ImageData,
) -> Box<Painter + Sync + Send + 'a> {
    Box::new(ImagePainter {
        image: image_data,
        transform: One::one(),
        transform_stack: Vec::new(),
        fill_color: RGBA::new(0.0, 0.0, 0.0, 1.0),
    })
}

impl<'a> Painter for ImagePainter<'a> {
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
            &mut SrgbRgba8RasterPort(self.image),
            self.transform,
            self.fill_color,
            layout,
            config,
        )
    }
}

use super::blend;

fn pack_u8x4(a: [u8; 4]) -> u32 {
    unsafe { ::std::mem::transmute(a) }
}

fn unpack_u8x4(a: u32) -> [u8; 4] {
    unsafe { ::std::mem::transmute(a) }
}

pub(crate) trait RasterPort {
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
        let stride = self.0.size().x;
        let offset_y = y * stride;

        debug_assert!(y < self.0.size().y);
        debug_assert!(x_range.end <= self.0.size().x);

        let pixels = self.0.pixels_u32_mut();

        let src_int = blend::srgb8_internal_mask(color, coverage);

        for pixel in pixels[offset_y + x_range.start..offset_y + x_range.end].iter_mut() {
            let dst_srgb = unpack_u8x4(*pixel);
            let out_srgb = blend::srgb8_alpha_over(src_int, dst_srgb);
            *pixel = pack_u8x4(out_srgb);
        }
    }
}
