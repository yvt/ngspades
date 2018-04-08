//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngspf_canvas as canvas;
extern crate ttf_noto_sans;

use canvas::text::*;

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
fn layout() {
    let mut config = FontConfig::new();
    config.insert(&load_behdad_regular(), 0, "Behdad", FontStyle::Normal, 400);
    config.insert(&load_noto_sans(), 0, "Noto Sans", FontStyle::Normal, 400);

    let para_style = ParagraphStyle::new();
    let layout =
        config.layout_point_text([("Hello, world! مرحبا ", ())][..].into(), &para_style);

    println!("{:#?}", layout);
}
