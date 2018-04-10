//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{Vector2, prelude::*};
use rgb::RGBA;
use std::ops::Range;

use painter::{Painter, text::rasterize_text_layout};
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

    fn fill_text_layout(&mut self, layout: &TextLayout, config: &FontConfig, colored: bool) {
        rasterize_text_layout(
            &mut SrgbRgba8RasterPort(self.image),
            self.transform,
            self.fill_color,
            layout,
            config,
            colored,
        )
    }
}

fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 {
        x * (1.0 / 12.92)
    } else {
        ((x + 0.055) * (1.0 / 1.055)).powf(2.4)
    }
}

fn linear_to_srgb(x: f32) -> f32 {
    if x < 0.0031308 {
        12.92 * x.max(0.0)
    } else {
        1.055 * x.min(1.0).powf(0.41666) - 0.055
    }
}

fn pack_u8x4(a: RGBA<f32>) -> u32 {
    let x = (a.b * 256.0).min(255.0) as u32;
    let y = (a.g * 256.0).min(255.0) as u32;
    let z = (a.r * 256.0).min(255.0) as u32;
    let w = (a.a * 256.0).min(255.0) as u32;
    x | (y << 8) | (z << 16) | (w << 24)
}

fn unpack_u8x4(a: u32) -> RGBA<f32> {
    let x = (a & 0x000000ffu32) as f32;
    let y = (a & 0x0000ff00u32) as f32;
    let z = (a & 0x00ff0000u32) as f32;
    let w = (a & 0xff000000u32) as f32;
    RGBA::new(
        z * (1.0 / 0x000000ffu32 as f32),
        y * (1.0 / 0x0000ff00u32 as f32),
        x * (1.0 / 0x00ff0000u32 as f32),
        w * (1.0 / 0xff000000u32 as f32),
    )
}

/// Blend `y` over `x`.
fn alpha_over_premul(x: RGBA<f32>, y: RGBA<f32>) -> RGBA<f32> {
    RGBA::new(
        x.r * (1.0 - y.a) + y.r,
        x.g * (1.0 - y.a) + y.g,
        x.b * (1.0 - y.a) + y.b,
        x.a * (1.0 - y.a) + y.a,
    )
}

fn to_alpha_premul(x: RGBA<f32>) -> RGBA<f32> {
    RGBA::new(x.r * x.a, x.g * x.a, x.b * x.a, x.a)
}

fn from_alpha_premul(x: RGBA<f32>) -> RGBA<f32> {
    let factor = if x.a == 0.0 { 1.0 } else { x.a.recip() };
    RGBA::new(x.r * factor, x.g * factor, x.b * factor, x.a)
}

pub(crate) trait RasterPort {
    fn size(&self) -> Vector2<usize>;

    /// `color` is not alpha pre-multiplied
    fn fill_span(&mut self, y: usize, x_range: Range<usize>, color: RGBA<f32>);
}

struct SrgbRgba8RasterPort<'a>(&'a mut ImageData);

impl<'a> RasterPort for SrgbRgba8RasterPort<'a> {
    fn size(&self) -> Vector2<usize> {
        self.0.size()
    }

    fn fill_span(&mut self, y: usize, x_range: Range<usize>, color: RGBA<f32>) {
        let stride = self.0.size().x;
        let offset_y = y * stride;

        debug_assert!(y < self.0.size().y);
        debug_assert!(x_range.end <= self.0.size().x);

        let pixels = self.0.pixels_u32_mut();

        let src_lin_pm = to_alpha_premul(color);

        for pixel in pixels[offset_y + x_range.start..offset_y + x_range.end].iter_mut() {
            let dst_srgb = unpack_u8x4(*pixel);
            let dst_lin = dst_srgb.map_rgb(srgb_to_linear);
            let dst_lin_pm = to_alpha_premul(dst_lin);

            let out_lin_pm = alpha_over_premul(dst_lin_pm, src_lin_pm);
            let out_lin = from_alpha_premul(out_lin_pm);
            let out_srgb = out_lin.map_rgb(linear_to_srgb);
            *pixel = pack_u8x4(out_srgb);
        }
    }
}
