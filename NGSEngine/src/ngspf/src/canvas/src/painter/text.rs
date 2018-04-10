//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::prelude::*;
use freetype::freetype::{FT_BBox, FT_Matrix};
use rgb::RGBA;
use std::os::raw::c_long;

use Affine2;
use painter::RasterPort;
use text::{FontConfig, TextLayout, ftutils::Library};

pub(crate) fn rasterize_text_layout<R: RasterPort>(
    port: &mut R,
    transform: Affine2<f64>,
    fill_color: RGBA<f32>,
    layout: &TextLayout,
    config: &FontConfig,
    colored: bool,
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
        ft_face.load_glyph(glyph.glyph_id, 0).unwrap();

        // Compute the transformation applied to `FT_Outline`.
        let tx = transform * Affine2::from_translation(glyph.position.to_vec())
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
        let color = if colored {
            unimplemented!()
        } else {
            fill_color
        };

        outline
            .render_direct(&ft_library, Some(clip_box), |y, spans| {
                for span in spans.iter() {
                    let x1 = span.x as usize;
                    let x2 = x1 + span.len as usize;
                    let mut span_color = color;
                    span_color.a *= span.coverage as f32 * (1.0 / 255.0);
                    port.fill_span(y as usize, x1..x2, span_color);
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
