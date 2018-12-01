//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate cgmath;
extern crate lipsum;
extern crate ngspf_canvas as canvas;
extern crate ttf_noto_sans;

use crate::canvas::{painter::*, text::*, *};
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
    painter.translate(Vector2::new(20.0, 240.0));
    for _ in 0..4 {
        painter.scale(2.0);
        painter.fill_text_layout(&layout, &config);
    }
}

#[test]
fn render_empty() {
    let mut config = FontConfig::new();
    config.insert(&load_noto_sans(), 0, "Noto Sans", FontStyle::Normal, 400);

    let para_style = ParagraphStyle::new();
    let layout = config.layout_point_text([("", ())][..].into(), &para_style);

    println!("{:#?}", layout);
    println!("Visual bounds = {:#?}", layout.visual_bounds());

    let mut image = ImageData::new(Vector2::new(640, 480), ImageFormat::SrgbRgba8);
    let mut painter = new_painter_for_image_data(&mut image);
    painter.fill_text_layout(&layout, &config);
}

fn layout_point_common(x: &str) {
    let mut config = FontConfig::new();
    config.insert(&load_noto_sans(), 0, "Noto Sans", FontStyle::Normal, 400);

    let para_style = ParagraphStyle::new();
    let layout = config.layout_point_text([(x, ())][..].into(), &para_style);

    println!("{:#?}", layout);
    println!("Visual bounds = {:#?}", layout.visual_bounds());
}

#[test]
fn layout_point_multi_line1() {
    layout_point_common("\n");
}
#[test]
fn layout_point_multi_line2() {
    layout_point_common("\n\n\n");
}
#[test]
fn layout_point_multi_line3() {
    layout_point_common("a\n\n\n");
}
#[test]
fn layout_point_multi_line4() {
    layout_point_common("\nb\n\n");
}
#[test]
fn layout_point_multi_line5() {
    layout_point_common("\n\nc\n");
}
#[test]
fn layout_point_multi_line6() {
    layout_point_common("\n\n\nd");
}

fn layout_area_common(x: &str) {
    let mut config = FontConfig::new();
    config.insert(&load_noto_sans(), 0, "Noto Sans", FontStyle::Normal, 400);

    let para_style = ParagraphStyle::new();
    let boundary = BoxBoundary::new(0.0..100.0);
    let layout = config.layout_area_text([(x, ())][..].into(), &para_style, &boundary);

    println!("{:#?}", layout);
    println!("Visual bounds = {:#?}", layout.visual_bounds());
}

#[test]
fn layout_area_multi_line1_no_wrap() {
    layout_area_common("\n");
}
#[test]
fn layout_area_multi_line2_no_wrap() {
    layout_area_common("\n\n\n");
}
#[test]
fn layout_area_multi_line3_no_wrap() {
    layout_area_common("a\n\n\n");
}
#[test]
fn layout_area_multi_line4_no_wrap() {
    layout_area_common("\nb\n\n");
}
#[test]
fn layout_area_multi_line5_no_wrap() {
    layout_area_common("\n\nc\n");
}
#[test]
fn layout_area_multi_line6_no_wrap() {
    layout_area_common("\n\n\nd");
}

#[test]
fn layout_area_no_wrap() {
    layout_area_common("Howdy!");
}

#[test]
fn layout_area_long() {
    layout_area_common(&lipsum::lipsum(16));
}

#[test]
fn layout_area_long_multi_line() {
    layout_area_common(
        &[
            lipsum::lipsum(16).as_str(),
            "\n",
            lipsum::lipsum(26).as_str(),
            "\n",
            lipsum::lipsum(64).as_str(),
        ]
        .concat(),
    );
}
