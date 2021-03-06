//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#[rustfmt::skip] // Removing the preceding `::` results in a compile error
use ::freetype::freetype::{self, FT_BBox, FT_Matrix};
use cgmath::prelude::*;
use rgb::RGBA;
use std::os::raw::c_long;

use crate::painter::RasterPort;
use crate::text::{ftutils::Library, FontConfig, TextLayout};
use crate::Affine2;

pub(crate) fn rasterize_text_layout<R: RasterPort>(
    port: &mut R,
    transform: Affine2<f64>,
    fill_color: RGBA<f32>,
    layout: &TextLayout,
    config: &FontConfig,
) {
    let ft_library = Library::global();

    let size = port.size();

    let clip_box = FT_BBox {
        xMin: 0,
        xMax: size.x as c_long,
        yMin: 0,
        yMax: size.y as c_long,
    };

    for glyph in layout.glyphs.iter() {
        let face = config.font_face(glyph.face_id);

        let ref ft_face = face.ft_face;
        let load_flags = freetype::FT_LOAD_NO_HINTING;
        ft_face.load_glyph(glyph.glyph_id, load_flags as _).unwrap();

        // Compute the transformation applied to `FT_Outline`.
        let tx = transform
            * Affine2::from_translation(glyph.position.to_vec())
            * Affine2::from_scale(glyph.scale * 64.0)
            * Affine2::from_nonuniform_scale(1.0, -1.0);
        let m = tx.as_matrix3();

        let mut outline = ft_face.glyph_slot_outline();

        outline.transform(&FT_Matrix {
            xx: to_fixed_16_16(m.x.x),
            xy: to_fixed_16_16(m.y.x),
            yx: to_fixed_16_16(m.x.y),
            yy: to_fixed_16_16(m.y.y),
        });
        outline.translate(to_fixed_26_6(m.z.x), to_fixed_26_6(m.z.y));

        // Compute the color of the glyph
        let color = glyph.color.unwrap_or(fill_color);

        let fast_color = port.to_fast_color(color);

        outline
            .render_direct(&ft_library, Some(clip_box), |y, spans| {
                for span in spans.iter() {
                    let x1 = span.x as usize;
                    let x2 = x1 + span.len as usize;
                    port.fill_span_cov(y as usize, x1..x2, fast_color, span.coverage);
                }
            })
            .unwrap();
    }
}

/// Convert a `f64` to `FT_Pos`.
fn to_fixed_26_6(x: f64) -> c_long {
    (x * 64.0) as c_long
}

/// Convert a `f64` to `FT_Fixed`.
fn to_fixed_16_16(x: f64) -> c_long {
    (x * 65536.0) as c_long
}
