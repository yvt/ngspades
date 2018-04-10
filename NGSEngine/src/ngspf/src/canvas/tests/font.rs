//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate cgmath;
extern crate ngspf_canvas as canvas;
extern crate ttf_noto_sans;

use canvas::{*, painter::*, text::*};
use cgmath::Vector2;

const BEHDAD_REGULAR: &[u8] = include_bytes!("fonts/Behdad-Regular.otf");

fn load_noto_sans() -> Font {
    Font::new(ttf_noto_sans::REGULAR).unwrap()
}

fn load_behdad_regular() -> Font {
    Font::new(BEHDAD_REGULAR).unwrap()
}

#[test]
fn create_config() {
    let mut config = FontConfig::new();
    config.insert(&load_noto_sans(), 0, "Noto Sans", FontStyle::Normal, 400);
}

#[test]
fn invalid_font() {
    Font::new(b"chimicherrychanga")
        .expect_err("Succeeded to create a Font from an invalid font data.");
}

#[test]
fn render_text() {
    let mut config = FontConfig::new();
    config.insert(&load_behdad_regular(), 0, "Behdad", FontStyle::Normal, 400);
    config.insert(&load_noto_sans(), 0, "Noto Sans", FontStyle::Normal, 400);

    let para_style = ParagraphStyle::new();
    let layout =
        config.layout_point_text([("Hello, world! مرحبا ", ())][..].into(), &para_style);

    println!("{:#?}", layout);
    println!("Visual bounds = {:#?}", layout.visual_bounds());

    let mut image = ImageData::new(Vector2::new(640, 480), ImageFormat::SrgbRgba8);
    let mut painter = new_painter_for_image_data(&mut image);
    painter.translate(Vector2::new(160.0, 240.0));
    painter.fill_text_layout(&layout, &config, false);
}
