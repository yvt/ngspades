//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(test)]
extern crate cgmath;
extern crate ngspf_canvas as canvas;
extern crate test;
extern crate ttf_noto_sans;

use canvas::{painter::*, text::*, *};
use cgmath::Vector2;

fn load_noto_sans() -> Font {
    Font::new(ttf_noto_sans::REGULAR).unwrap()
}

#[bench]
fn layout_simple(b: &mut test::Bencher) {
    let mut config = FontConfig::new();
    config.insert(&load_noto_sans(), 0, "Noto Sans", FontStyle::Normal, 400);

    let para_style = ParagraphStyle::new();
    b.iter(move || {
        config.layout_point_text([("Hello, world!", ())][..].into(), &para_style);
    });
}

#[bench]
fn render_text(b: &mut test::Bencher) {
    let mut config = FontConfig::new();
    config.insert(&load_noto_sans(), 0, "Noto Sans", FontStyle::Normal, 400);

    let para_style = ParagraphStyle::new();
    let layout = config.layout_point_text([("Hello, world!", ())][..].into(), &para_style);

    let mut image = ImageData::new(Vector2::new(640, 480), ImageFormat::SrgbRgba8);
    let mut painter = new_painter_for_image_data(&mut image);
    painter.translate(Vector2::new(40.0, 400.0));
    painter.scale(8.0);
    b.iter(move || {
        painter.fill_text_layout(&layout, &config);
    });
}
