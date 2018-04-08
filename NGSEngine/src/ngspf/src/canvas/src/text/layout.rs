//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Text layout engine.
//!
//! # Terminology
//!
//! **Character styles** are properties associated with each code point that
//! control their appearance, including their color, decorative elements (e.g.,
//! underline), and the font used to render the text.
//!
//! **Font styles** are a subset of character styles that affect the choice of
//! the font face used to render the text. Ligatures and font shaping cannot
//! occur accross the font style boundaries.
//!
//! # Text layouting
//!
//! The text layouting basically proceeds as the following:
//!
//!  1. The application provides a string representing the input paragraph. The
//!     input string can be either a plain text (`String`) or an attributed text
//!     (`attrtext::Text`). The application also provides a `ParagraphStyle` and
//!     `Boundary` (only for area type).
//!
//!  2. The final font style is computed for each character (precisely, single
//!     code point or TODO: multiple code points to support variation sequences).
//!     The BiDi level of each byte in the text is analyzed by [`unicode-bidi`].
//!     The paragraph embedding level is determined from
//!     `ParagraphStyle::direction`.
//!     Finally, the text is broken into a series of *shaping clusters*, each of
//!     which contains a sequence of characters sharing an identical set of
//!     font styles, the same BiDi run direction, and some other properties.
//!
//!  3. Font shaping is performed by HarfBuzz on each shaping cluster.
//!
//!  4. The result is examined for glyphs lying over character style boundaries.
//!     If such glyphs were found, explicit invisible separators are inserted
//!     to prevent the formation of ligatures and font shaping is tried again.
//!     (Theoretically, this modification might cause character style boundary
//!     violation to occur on other places, but currently we rule out such cases
//!     as we assume that such cases are rare.)
//!
//!  5. The line break opportunities, a set of positions within the text where
//!     soft wrapping is allowed, are computed using the Unicode line breaking
//!     algorithm [[UAX14]]. The mandatory line break points are computed as
//!     well.
//!
//!  6. Word wrapping is performed using the information obtained in the
//!     previous step. The output contains a byte range for each line generated.
//!
//!  7. For every line, a sequence of shaping clusters (after shaping), ordered
//!     by the display order as determined by rules L1–L4 (as implemented by
//!     `unicode-bidi`), is generated.
//!
//!  8. Alignment and justification is performed on every line. Justification
//!     follows the specification by *Section 3*, *Introduction*, in [[UAX14]].
//!     After that, the layout of the text is finalized.
//!
//! [`unicode-bidi`]: https://crates.io/crates/unicode-bidi
//! [UAX14]: http://www.unicode.org/reports/tr14/
use attrtext::{Override, Text};
use harfbuzz;
use std::borrow::Cow;
use std::ops::Range;
use unicode_bidi::{self, BidiInfo, ParagraphInfo};
use xi_unicode::LineBreakIterator;

use super::{hbutils, AsElementRef, Boundary, CharStyle, Direction, ElementRef, FontConfig,
            FontFaceId, FontFaceProps, ForeignObject, ParagraphStyle, TextAlign, FONT_SCALE};

const FOREIGN_MARKER_STR: &str = "\u{f8ff}";

impl FontConfig {
    /// Layout the given text as area type.
    pub fn layout_area_text<S, A, B>(
        &self,
        text: &Text<S, A>,
        para_style: &ParagraphStyle,
        boundary: &B,
    ) -> TextLayout
    where
        S: AsElementRef,
        CharStyle: Override<A>,
        B: Boundary,
    {
        self.layout_text(text, para_style, Some(boundary))
    }

    /// Layout the given text as point type.
    pub fn layout_point_text<S, A>(
        &self,
        text: &Text<S, A>,
        para_style: &ParagraphStyle,
    ) -> TextLayout
    where
        S: AsElementRef,
        CharStyle: Override<A>,
    {
        self.layout_text::<S, A, !>(text, para_style, None)
    }

    /// Layout the given text.
    fn layout_text<S, A, B>(
        &self,
        text: &Text<S, A>,
        para_style: &ParagraphStyle,
        boundary: Option<&B>,
    ) -> TextLayout
    where
        S: AsElementRef,
        CharStyle: Override<A>,
        B: Boundary,
    {
        let flattened = flatten_text(text);
        let ref flattened = *flattened; // let's be kind to the optimizer

        assert!(
            flattened.len() as u32 as usize == flattened.len(),
            "The input text is too long."
        );

        // Determine the Bidi level for each byte.
        let default_bidi_level = match para_style.direction {
            Direction::LeftToRight | Direction::TopToBottom => unicode_bidi::Level::ltr(),
            Direction::RightToLeft | Direction::BottomToTop => unicode_bidi::Level::rtl(),
        };
        let bidi_info = BidiInfo::new(&flattened, Some(default_bidi_level));
        let bidi_para_info = ParagraphInfo {
            range: 0..flattened.len(),
            level: default_bidi_level,
        };

        let is_vertical = para_style.direction.is_vertical();

        // Break the text into shaping clusters.
        #[derive(Debug)]
        struct ShapingCluster<'a> {
            /// The starting byte offset in `flattened`
            start_flattened: usize,
            contents: ClusterContents<'a>,
        }
        #[derive(Debug)]
        enum ClusterContents<'a> {
            /// One or more text spans.
            Text(ShapingProps),
            /// A foreign object.
            Foreign(&'a ForeignObject),
        }
        #[derive(Debug, Clone, Copy, PartialEq)]
        struct ShapingProps {
            face_id: Option<FontFaceId>,
            size: f64,
            bidi_level: unicode_bidi::Level,
            script: hbutils::Script,
            language: hbutils::Language,
        }

        let shaping_clusters = {
            let mut clusters = Vec::new();

            let mut index_input: usize = 0;
            let mut index_flattened: usize = 0;

            let mut selector = self.selector();

            let mut last_text = None;

            for &(ref span, ref attr) in text.iter() {
                match span.as_element_ref() {
                    ElementRef::Foreign(x) => {
                        clusters.push(ShapingCluster {
                            start_flattened: index_flattened,
                            contents: ClusterContents::Foreign(x),
                        });
                        index_input += x.len();
                        index_flattened += FOREIGN_MARKER_STR.len();
                        last_text = None;
                    }
                    ElementRef::Text(x) => {
                        let char_style = para_style.char_style.override_with(attr);
                        selector.set_font_families(&char_style.font_family);

                        let face_props = FontFaceProps::from_char_style(&char_style);
                        let size = char_style.font_size.expect("font size is missing");
                        let script = char_style.script.expect("script is missing");
                        let language = char_style.language.expect("language is missing");

                        for c in x.chars() {
                            let face_id = selector.optimal_font_face(c, &face_props);

                            let bidi_level = bidi_info.levels[index_flattened];
                            let shaping_props = ShapingProps {
                                face_id,
                                size,
                                bidi_level,
                                script,
                                language,
                            };

                            if Some(shaping_props) != last_text {
                                clusters.push(ShapingCluster {
                                    start_flattened: index_flattened,
                                    contents: ClusterContents::Text(shaping_props),
                                });
                                last_text = Some(shaping_props);
                            }

                            let len_utf8 = c.len_utf8();
                            index_input += len_utf8;
                            index_flattened += len_utf8;
                        }
                    }
                }
            }

            clusters
        };

        // Shape each shaping cluster.
        use iterutils::IterUtils;
        let hb_buffers: Vec<_> = shaping_clusters
            .iter()
            // Generate `flattened_range`, a byte range in `flattened`
            .with_range(flattened.len(), |x| x.start_flattened)
            .map(|(flattened_range, cluster)| {
                if let ClusterContents::Text(shaping_props) = cluster.contents {
                    let face = if let Some(face_id) = shaping_props.face_id {
                        self.font_face(face_id)
                    } else {
                        // Unrenderable text contents (for which
                        // `optimal_font_face` returned `None`)
                        return None;
                    };

                    let mut hb_buffer = hbutils::Buffer::new();
                    hb_buffer.set_content_type(harfbuzz::HB_BUFFER_CONTENT_TYPE_UNICODE);
                    hb_buffer.set_language(shaping_props.language);
                    hb_buffer.set_script(shaping_props.script);
                    hb_buffer.set_direction(match (is_vertical, shaping_props.bidi_level.is_rtl()) {
                        (false, false) => harfbuzz::HB_DIRECTION_LTR,
                        (false, true) => harfbuzz::HB_DIRECTION_RTL,
                        (true, false) => harfbuzz::HB_DIRECTION_TTB,
                        (true, true) => harfbuzz::HB_DIRECTION_BTT,
                    });
                    for (i, c) in flattened[flattened_range.clone()].char_indices() {
                        // The cluster value is a byte index into `flattened`
                        hb_buffer.add(c, (i + flattened_range.start) as u32);
                    }

                    face.hb_font.shape(&mut hb_buffer);

                    // Since the cluster level of `hb_buffer` is zero (default),
                    // the post-shaping cluster values are guaranteed to be
                    // monotonically increasing.
                    // If the span is RTL, then the order of `hb_buffer` is also
                    // reversed.

                    // TODO: Check character style boundary violation
                    Some(hb_buffer)
                } else {
                    // Foreign contents
                    None
                }
            })
            .collect();

        // Lines
        struct Line {
            start_flattened: usize,
            x_coord_range: Range<f64>,
            y_coord: f64,
        }
        let mut lines = Vec::new();

        if let Some(boundary) = boundary {
            // TODO: Word-wrapping
            unimplemented!();
        } else {
            // Point type - simply break lines at hard breaks
            lines.push(Line {
                start_flattened: 0,
                x_coord_range: 0.0..0.0,
                y_coord: 0.0,
            });
            for (i, hard) in LineBreakIterator::new(flattened) {
                if hard {
                    // Do not include end-of-text in `lines`.
                    if i < flattened.len() {
                        lines.push(Line {
                            start_flattened: i,
                            x_coord_range: 0.0..0.0,
                            y_coord: 0.0, // TODO: compute Y coord
                        });
                    }
                }
            }
        }

        fn clip_shaping_cluster_glyphs(
            hb_buffer: &hbutils::Buffer,
            flattened_range: &Range<usize>,
            is_rtl: bool,
        ) -> Range<usize> {
            let glyph_infos = hb_buffer.glyph_infos();
            let mut glyph_range = 0..glyph_infos.len();

            let flattened_range = flattened_range.start as u32..flattened_range.end as u32;

            while glyph_range.start < glyph_range.end {
                if is_rtl {
                    if glyph_infos[glyph_range.start].cluster < flattened_range.end {
                        break;
                    }
                } else {
                    if glyph_infos[glyph_range.start].cluster >= flattened_range.start {
                        break;
                    }
                }
                glyph_range.start += 1;
            }

            while glyph_range.start < glyph_range.end {
                if is_rtl {
                    if glyph_infos[glyph_range.end - 1].cluster >= flattened_range.start {
                        break;
                    }
                } else {
                    if glyph_infos[glyph_range.end - 1].cluster < flattened_range.end {
                        break;
                    }
                }
                glyph_range.end -= 1;
            }

            glyph_range
        }

        // Layout each line
        let mut output_glyphs = Vec::new();
        let output_lines: Vec<_> = lines.iter()
            // Generate `flattened_range`, a byte range in `flattened`
            .with_range(flattened.len(), |x| x.start_flattened)
            .map(|(mut flattened_range, line)| {
                // Ignore trailing spaces (but not line break characters nor EOT)
                if flattened_range.end < flattened.len() {
                    let bytes = flattened.as_bytes();
                    while flattened_range.end > flattened_range.start {
                        if bytes[flattened_range.end - 1] == 0x20 {
                            // U+0020 Space
                            flattened_range.end -= 1;
                        } else if flattened_range.end > flattened_range.start + 1 &&
                            bytes[flattened_range.end - 1] == 0xa0 &&
                            bytes[flattened_range.end - 2] == 0xc2 {
                            // U+00A0 No-Break Space
                            flattened_range.end -= 2;
                        } else {
                            break;
                        }
                    }
                }

                // Compute the display order
                let (levels, level_runs) = bidi_info.visual_runs(&bidi_para_info, flattened_range.clone());

                // Collect shaping clusters in the line
                struct LineCluster {
                    /// Index into `shaping_clusters` and `hb_buffers`
                    shaping_cluster_id: usize,
                    glyph_range: Range<usize>,
                }
                let mut clusters = Vec::new();

                for level_run in level_runs.iter() {
                    let is_rtl = levels[level_run.start].is_rtl();

                    if is_rtl {
                        let mut i = match shaping_clusters.binary_search_by_key(&level_run.end, |x| x.start_flattened) {
                            Ok(i) => i,
                            Err(i) => i,
                        };
                        loop {
                            i -= 1;
                            clusters.push(LineCluster {
                                shaping_cluster_id: i,
                                glyph_range: if let Some(ref hb_buffer) = hb_buffers[i] {
                                    clip_shaping_cluster_glyphs(hb_buffer, level_run, is_rtl)
                                } else {
                                    0..0
                                }
                            });
                            if shaping_clusters[i].start_flattened <= level_run.start {
                                break;
                            }
                        }
                    } else {
                        let mut i = match shaping_clusters.binary_search_by_key(&level_run.start, |x| x.start_flattened) {
                            Ok(i) => i,
                            Err(i) => i - 1,
                        };
                        while i < shaping_clusters.len() {
                            if shaping_clusters[i].start_flattened >= level_run.end {
                                break;
                            }
                            clusters.push(LineCluster {
                                shaping_cluster_id: i,
                                glyph_range: if let Some(ref hb_buffer) = hb_buffers[i] {
                                    clip_shaping_cluster_glyphs(hb_buffer, level_run, is_rtl)
                                } else {
                                    0..0
                                }
                            });
                            i += 1;
                        }
                    }
                }

                // Compute the position of the line
                let mut offs = [0.0f64; 2];

                let line_width = || -> f64 {
                    clusters.iter().map(|cluster| {
                        let ref shaping_cluster = shaping_clusters[cluster.shaping_cluster_id];
                        let ref hb_buffer = hb_buffers[cluster.shaping_cluster_id];

                        if cluster.glyph_range.len() > 0 {
                            let glyph_positions = hb_buffer.as_ref().unwrap().glyph_positions();

                            let cluster_width = glyph_positions[cluster.glyph_range.clone()].iter().map(|x| if is_vertical {
                                x.y_advance
                            } else {
                                x.x_advance
                            }).sum::<i32>();

                            // Multiply by the font size
                            let font_size = if let ClusterContents::Text(ref props) = shaping_cluster.contents {
                                props.size
                            } else {
                                unreachable!()
                            };

                            cluster_width as f64 * (1.0 / FONT_SCALE) * font_size
                        } else {
                            match shaping_cluster.contents {
                                ClusterContents::Text(_) => 0.0,
                                ClusterContents::Foreign(x) => {
                                    let extents = x.extents();
                                    if is_vertical {
                                        extents[1]
                                    } else {
                                        extents[0]
                                    }
                                }
                            }
                        }
                    }).sum()
                };

                // TODO: justification

                match (default_bidi_level.is_rtl(), para_style.text_align) {
                    (false, TextAlign::Start) |
                    (false, TextAlign::Justify) |
                    (false, TextAlign::JustifyAll) |
                    (true, TextAlign::End) => {
                        // Left-aligned
                        offs[0] = line.x_coord_range.start;
                        offs[1] = line.y_coord;
                    }
                    (false, TextAlign::End) |
                    (true, TextAlign::Start) |
                    (true, TextAlign::Justify) |
                    (true, TextAlign::JustifyAll) => {
                        // Right-aligned
                        offs[0] = line.x_coord_range.end - line_width();
                        offs[1] = line.y_coord;
                    }
                    (_, TextAlign::Center) => {
                        // Center-aligned
                        offs[0] = line.x_coord_range.start + (line.x_coord_range.end - line_width()) * 0.5;
                        offs[1] = line.y_coord;
                    }
                }

                if is_vertical {
                    offs.swap(0, 1);
                    offs[0] = -offs[0];
                }

                // TODO: Export selection rectangles

                // Place glyphs
                for cluster in clusters.iter() {
                    let ref shaping_cluster = shaping_clusters[cluster.shaping_cluster_id];
                    let ref hb_buffer = hb_buffers[cluster.shaping_cluster_id];

                    if cluster.glyph_range.len() > 0 {
                        let glyph_infos = hb_buffer.as_ref().unwrap().glyph_infos();
                        let glyph_positions = hb_buffer.as_ref().unwrap().glyph_positions();
                        let ref range = cluster.glyph_range;

                        // Multiply by the font size
                        let props = if let ClusterContents::Text(ref props) = shaping_cluster.contents {
                            props
                        } else {
                            unreachable!()
                        };
                        let scale = props.size * (1.0 / FONT_SCALE);

                        for (info, pos) in glyph_infos[range.clone()].iter()
                            .zip(glyph_positions[range.clone()].iter()) {

                            output_glyphs.push(GlyphLayout {
                                position: [
                                    offs[0] + pos.x_offset as f64 * scale,
                                    offs[1] + pos.y_offset as f64 * scale,
                                ],
                                scale,
                                face_id: props.face_id.unwrap(),
                                glyph_id: info.codepoint,
                            });

                            offs[0] += pos.x_advance as f64 * scale;
                            offs[1] += pos.y_advance as f64 * scale;
                        }
                    } else {
                        match shaping_cluster.contents {
                            ClusterContents::Text(_) => {},
                            ClusterContents::Foreign(x) => {
                                let extents = x.extents();
                                // TODO: Export the position of foreign objects
                                if is_vertical {
                                    offs[1] += extents[1];
                                } else {
                                    offs[0] += extents[0];
                                }
                            }
                        }
                    }
                }

                LineLayout{}
            })
            .collect();

        TextLayout {
            lines: output_lines,
            glyphs: output_glyphs,
        }
    }
}

/// Flatten a `attrtext::Text` into a `String`, substituting every foreign
/// object with a single private-use character.
///
/// Obviously, the resulting text would have a different length from that of
/// the input text.
fn flatten_text<S: AsElementRef, A>(text: &Text<S, A>) -> Cow<str> {
    let needs_owned = text.iter()
        .enumerate()
        .any(|(i, x)| i > 0 || x.0.as_element_ref().foreign().is_some());

    if needs_owned {
        use itertools::Itertools;
        let concatenated = text.iter()
            .map(|x| match x.0.as_element_ref() {
                ElementRef::Text(x) => x,
                ElementRef::Foreign(_) => FOREIGN_MARKER_STR,
            })
            .join("");

        Cow::Owned(concatenated)
    } else {
        match text.iter().next() {
            Some(&(ref text, _)) => Cow::Borrowed(text.as_element_ref().text().unwrap()),
            None => Cow::Borrowed(""),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextLayout {
    lines: Vec<LineLayout>,
    glyphs: Vec<GlyphLayout>,
}

#[derive(Debug, Clone, Copy)]
pub struct LineLayout {
    // nothing so far
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphLayout {
    position: [f64; 2],
    scale: f64,
    face_id: FontFaceId,
    glyph_id: u32,
}
