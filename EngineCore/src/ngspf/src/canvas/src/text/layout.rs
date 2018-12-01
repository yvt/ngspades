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
use attrtext::{text::Cursor, Override, Span, Text};
use cgmath::{Point2, Vector2};
use rgb::RGBA;
use std::borrow::Cow;
use std::ops::Range;
use unicode_bidi::{self, BidiInfo, ParagraphInfo};
use xi_unicode::LineBreakIterator;

use super::{
    hbutils, AsCharStyleRef, AsElementRef, Boundary, Direction, ElementRef, FontConfig, FontFaceId,
    FontFaceProps, ForeignObject, ParagraphStyle, TextAlign, WordWrapMode, FONT_SCALE,
};

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
        S: AsElementRef + Span,
        A: AsCharStyleRef,
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
        S: AsElementRef + Span,
        A: AsCharStyleRef,
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
        S: AsElementRef + Span,
        A: AsCharStyleRef,
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
            /// The starting position in `text`
            start_text: Cursor,
            contents: ClusterContents<'a>,
        }
        #[derive(Debug)]
        enum ClusterContents<'a> {
            /// One or more text spans.
            Text(ShapingProps),
            /// A foreign object. The second value is font size.
            Foreign(&'a ForeignObject, f64),
        }
        #[derive(Debug, Clone, Copy, PartialEq)]
        struct ShapingProps {
            face_id: Option<FontFaceId>,
            size: f64,
            bidi_level: unicode_bidi::Level,
            script: Option<hbutils::Script>,
            language: Option<hbutils::Language>,
        }

        let shaping_clusters = {
            let mut clusters = Vec::new();

            let mut index_flattened: usize = 0;

            let mut selector = self.selector();

            let mut last_text = None;

            for run in text.runs() {
                let span = *run.span();
                let attr = *run.attribute();

                let char_style = para_style
                    .char_style
                    .as_char_style_ref()
                    .override_with(&attr.as_char_style_ref());

                let size = char_style.font_size.expect("font size is missing");

                match span.as_element_ref() {
                    ElementRef::Foreign(x) => {
                        clusters.push(ShapingCluster {
                            start_flattened: index_flattened,
                            start_text: run.cursor().start,
                            contents: ClusterContents::Foreign(x, size),
                        });
                        index_flattened += FOREIGN_MARKER_STR.len();
                        last_text = None;
                    }
                    ElementRef::Text(x) => {
                        selector.set_font_families(char_style.font_family);

                        let face_props = FontFaceProps::from_char_style(&char_style);
                        let script = char_style.script;
                        let language = char_style.language;

                        for (offset, c) in x.char_indices() {
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
                                    start_text: text
                                        .offset(run.cursor().start, offset as isize)
                                        .unwrap(),
                                    contents: ClusterContents::Text(shaping_props),
                                });
                                last_text = Some(shaping_props);
                            }

                            let len_utf8 = c.len_utf8();
                            index_flattened += len_utf8;
                        }
                    }
                }
            }

            clusters
        };

        // Shape each shaping cluster.
        use crate::iterutils::IterUtils;
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
                    hb_buffer.set_content_type(harfbuzz_sys::HB_BUFFER_CONTENT_TYPE_UNICODE);
                    if let Some(x) = shaping_props.language {
                        hb_buffer.set_language(x);
                    }
                    if let Some(x) = shaping_props.script {
                        hb_buffer.set_script(x);
                    }
                    hb_buffer.set_direction(
                        match (is_vertical, shaping_props.bidi_level.is_rtl()) {
                            (false, false) => harfbuzz_sys::HB_DIRECTION_LTR,
                            (false, true) => harfbuzz_sys::HB_DIRECTION_RTL,
                            (true, false) => harfbuzz_sys::HB_DIRECTION_TTB,
                            (true, true) => harfbuzz_sys::HB_DIRECTION_BTT,
                        },
                    );
                    for (i, c) in flattened[flattened_range.clone()].char_indices() {
                        // FIXME: Probably we should skip other control characters as well
                        if c == '\r' || c == '\n' {
                            continue;
                        }
                        // The cluster value is a byte index into `flattened`
                        hb_buffer.add(c, (i + flattened_range.start) as u32);
                    }

                    if shaping_props.language.is_none() || shaping_props.script.is_none() {
                        hb_buffer.guess_segment_properties();
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
            /// Container width obtained by `Boundary::line_range`.
            x_coord_range: Range<f64>,
            /// The Y coordinate of the baseline.
            y_coord: f64,
            /// Is this line terminated by a hard line break? End-of-text counts
            /// as a hard line break.
            hard_break: bool,
        }
        let mut lines = Vec::new();

        if let Some(boundary) = boundary {
            // Area type - perform word-wrapping
            use super::wordwrap;
            use itertools::unfold;
            use std::iter::Peekable;

            let mut start = LineBreakIterator::new(flattened);
            let mut start_flattened = 0;

            // `Iterator<Item = (Range<usize>, (&ShapingCluster, &Option<hbutils::Buffer>))>`
            let mut clusters = shaping_clusters.iter().zip(hb_buffers.iter()).peekable();

            fn get_cluster_start_glyph_index(
                (shaping_cluster, hb_buffer): (&ShapingCluster, &Option<hbutils::Buffer>),
            ) -> usize {
                if let ClusterContents::Text(ref props) = shaping_cluster.contents {
                    if props.bidi_level.is_rtl() {
                        hb_buffer.as_ref().unwrap().glyph_infos().len()
                    } else {
                        0
                    }
                } else {
                    0
                }
            }

            #[derive(Debug, Copy, Clone)]
            enum GlyphOrForeign<'a> {
                Glyph(
                    &'a harfbuzz_sys::hb_glyph_info_t,
                    &'a harfbuzz_sys::hb_glyph_position_t,
                    &'a ShapingProps,
                ),
                Foreign(&'a ForeignObject, f64),
            }

            /// Advance `(clusters, glyph_index)` until a glyph with the `flattened`
            /// offset value `index_flattened` is met.
            ///
            /// Returns whether `index_flattened` fell into a glyph boundary.
            ///
            /// Calls `cb_glyph` for each glyph passed by. Calls `cb_cluster`
            /// for each cluster passed by.
            fn clusters_advance_until<'a, I, F1, F2>(
                clusters: &mut Peekable<I>,
                glyph_index: &mut usize,
                index_flattened: usize,
                mut cb_glyph: F1,
                mut cb_cluster: F2,
            ) -> bool
            where
                I: Iterator<Item = (&'a ShapingCluster<'a>, &'a Option<hbutils::Buffer>)>,
                F1: FnMut(GlyphOrForeign),
                F2: FnMut(&ShapingCluster, &Option<hbutils::Buffer>),
            {
                loop {
                    if let Some(cluster) = clusters.peek() {
                        let &(shaping_cluster, hb_buffer) = cluster;

                        cb_cluster(shaping_cluster, hb_buffer);

                        if shaping_cluster.start_flattened >= index_flattened {
                            return shaping_cluster.start_flattened == index_flattened;
                        }

                        match shaping_cluster.contents {
                            ClusterContents::Text(ref props) => {
                                if let &Some(ref hb_buffer) = hb_buffer {
                                    let glyph_infos = hb_buffer.glyph_infos();
                                    let glyph_positions = hb_buffer.glyph_positions();

                                    assert!(glyph_positions.len() == glyph_infos.len());

                                    if props.bidi_level.is_rtl() {
                                        assert!(*glyph_index <= glyph_infos.len());
                                        while *glyph_index > 0 {
                                            let i = glyph_infos[*glyph_index - 1].cluster as usize;
                                            if i >= index_flattened {
                                                return i == index_flattened;
                                            }

                                            *glyph_index -= 1;
                                            cb_glyph(GlyphOrForeign::Glyph(
                                                &glyph_infos[*glyph_index],
                                                &glyph_positions[*glyph_index],
                                                props,
                                            ));
                                        }
                                    } else {
                                        while *glyph_index < glyph_infos.len() {
                                            let i = glyph_infos[*glyph_index].cluster as usize;
                                            if i >= index_flattened {
                                                return i == index_flattened;
                                            }

                                            cb_glyph(GlyphOrForeign::Glyph(
                                                &glyph_infos[*glyph_index],
                                                &glyph_positions[*glyph_index],
                                                props,
                                            ));
                                            *glyph_index += 1;
                                        }
                                    }
                                }
                            }
                            ClusterContents::Foreign(foreign, size) => {
                                cb_glyph(GlyphOrForeign::Foreign(foreign, size));
                            }
                        }
                    } else {
                        // By definition we're supposed to return `true` if `index_flattened`
                        // is `flattened.len()`, but on practical it doesn't matter
                        return false;
                    }

                    clusters.next();
                    *glyph_index = if let Some(&cluster) = clusters.peek() {
                        get_cluster_start_glyph_index(cluster)
                    } else {
                        0
                    };
                }
            }

            let mut glyph_index = if let Some(&cluster) = clusters.peek() {
                get_cluster_start_glyph_index(cluster)
            } else {
                0
            };

            let mut initial_y = 0.0f64;

            while start_flattened < flattened.len() {
                struct State<'a, T> {
                    active: bool,
                    start_flattened: usize,
                    line_break_it: LineBreakIterator<'a>,
                    cluster_it: T,
                    glyph_index: usize,
                }
                // Construct a list of `Word`s in the current line.
                //
                // Since `LineBreakIterator` doesn't return the starting point
                // of each word, we have to maintain it using the state variable
                // of `unfold`.
                let mut words = unfold(
                    State {
                        active: true,
                        start_flattened,
                        line_break_it: start,
                        cluster_it: clusters,
                        glyph_index,
                    },
                    |st| {
                        if !st.active {
                            return None;
                        }

                        let start_flattened = st.start_flattened;

                        let mut word_width = 0.0f64;
                        let mut spacing_width = 0.0f64;
                        let mut max_size = 0.0f64;

                        // Locate the next break opportunity
                        let (end_flattened, hard) = loop {
                            let (end_flattened, hard) = st.line_break_it.next().unwrap();

                            // Count succeeding whitespaces
                            let mut start_ws = end_flattened;
                            while start_ws > 0 && flattened.as_bytes()[start_ws - 1] == 0x20 {
                                start_ws -= 1;
                            }

                            // Check if we can really insert a break at `end_flattened`
                            // Specifically, are we at a middle of a glyph there?
                            //
                            // Calculate the word width at the same time.
                            let mut okay = clusters_advance_until(
                                &mut st.cluster_it,
                                &mut st.glyph_index,
                                end_flattened,
                                |glyph_or_foreign| match glyph_or_foreign {
                                    GlyphOrForeign::Glyph(glyph_info, glyph_pos, shaping_props) => {
                                        let scale = shaping_props.size * (1.0 / FONT_SCALE);

                                        let advance = if is_vertical {
                                            glyph_pos.y_advance
                                        } else {
                                            glyph_pos.x_advance
                                        };

                                        let advance = advance as f64 * scale;

                                        let i = glyph_info.cluster as usize;
                                        if i >= start_ws {
                                            spacing_width += advance;
                                        } else {
                                            word_width += advance;
                                        }
                                    }
                                    GlyphOrForeign::Foreign(foreign, _) => {
                                        let extents = foreign.extents();
                                        if is_vertical {
                                            word_width += extents[1];
                                        } else {
                                            word_width += extents[0];
                                        }
                                    }
                                },
                                |shaping_cluster, _| {
                                    // Record the maximum font size in this range
                                    match shaping_cluster.contents {
                                        ClusterContents::Text(ref props) => {
                                            max_size = max_size.max(props.size);
                                        }
                                        ClusterContents::Foreign(_, size) => {
                                            max_size = max_size.max(size);
                                        }
                                    }
                                },
                            );

                            if okay || hard {
                                break (end_flattened, hard);
                            } else {
                                word_width += spacing_width;
                                spacing_width = 0.0;
                            }
                        };

                        st.start_flattened = end_flattened;
                        if hard {
                            // Found a hard break; stop iteration
                            st.active = false;
                        }

                        let line_height = (max_size * para_style.line_height.factor)
                            .max(para_style.line_height.minimum);

                        Some(wordwrap::Word {
                            width: word_width,
                            spacing: spacing_width,
                            line_height,
                            value: start_flattened,
                        })
                    },
                );

                // Perform word-wrapping
                match para_style.word_wrap_mode {
                    WordWrapMode::MinNumLines => {
                        for line in wordwrap::word_wrap_greedy(&mut words, boundary, initial_y) {
                            initial_y = line.y_coord;
                            lines.push(Line {
                                start_flattened: line.start_value,
                                x_coord_range: line.x_coord_range,
                                y_coord: line.y_coord,
                                hard_break: false,
                            });
                        }
                    }
                    WordWrapMode::MinRaggedness => unimplemented!(),
                }

                lines.last_mut().unwrap().hard_break = true;

                // Find the next hard break and go on to the next line
                let state = words.state;
                assert!(!state.active);
                start_flattened = state.start_flattened;
                start = state.line_break_it;
                clusters = state.cluster_it;
                glyph_index = state.glyph_index;
            }
        } else {
            // Point type - simply break lines at hard breaks
            lines.push(Line {
                start_flattened: 0,
                x_coord_range: 0.0..0.0,
                y_coord: 0.0,
                hard_break: true,
            });
            for (i, hard) in LineBreakIterator::new(flattened) {
                if hard {
                    // Do not include end-of-text in `lines`.
                    if i < flattened.len() {
                        lines.push(Line {
                            start_flattened: i,
                            x_coord_range: 0.0..0.0,
                            y_coord: 0.0, // set later
                            hard_break: true,
                        });
                    }
                }
            }

            // Compute the Y coord of lines
            if lines.len() > 1 {
                let mut clusters = shaping_clusters
                    .iter()
                    .with_range(flattened.len(), |x| x.start_flattened)
                    .peekable();

                let lines = lines
                    .iter_mut()
                    .with_range(flattened.len(), |x| x.start_flattened);

                let mut initial_y = 0.0f64;

                for (line_range, line) in lines {
                    let mut max_size = 0.0f64;

                    while let Some((range, shaping_cluster)) = clusters.peek().cloned() {
                        if range.end >= line_range.end {
                            break;
                        }

                        match shaping_cluster.contents {
                            ClusterContents::Text(ref props) => {
                                max_size = max_size.max(props.size);
                            }
                            ClusterContents::Foreign(_, size) => {
                                max_size = max_size.max(size);
                            }
                        }

                        clusters.next();
                    }

                    if line_range.start == 0 {
                        // Skip the first line
                        continue;
                    }

                    let line_height = (max_size * para_style.line_height.factor)
                        .max(para_style.line_height.minimum);
                    initial_y += line_height;
                    line.y_coord = initial_y;
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
        let output_lines: Vec<_> = lines
            .iter()
            // Generate `flattened_range`, a byte range in `flattened`
            .with_range(flattened.len(), |x| x.start_flattened)
            .map(|(mut flattened_range, line)| {
                // Ignore trailing spaces (but not line break characters nor EOT)
                if !line.hard_break {
                    let bytes = flattened.as_bytes();
                    while flattened_range.end > flattened_range.start {
                        if bytes[flattened_range.end - 1] == 0x20 {
                            // U+0020 Space
                            flattened_range.end -= 1;
                        } else {
                            break;
                        }
                    }
                }

                // Compute the display order
                let (levels, level_runs) = if flattened_range.len() != 0 {
                    bidi_info.visual_runs(&bidi_para_info, flattened_range.clone())
                } else {
                    (Vec::new(), Vec::new())
                };

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
                        let mut i = match shaping_clusters
                            .binary_search_by_key(&level_run.end, |x| x.start_flattened)
                        {
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
                                },
                            });
                            if shaping_clusters[i].start_flattened <= level_run.start {
                                break;
                            }
                        }
                    } else {
                        let mut i = match shaping_clusters
                            .binary_search_by_key(&level_run.start, |x| x.start_flattened)
                        {
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
                                },
                            });
                            i += 1;
                        }
                    }
                }

                // Compute the position of the line
                let mut offs = [0.0f64; 2];

                let mut line_width: f64 = clusters
                    .iter()
                    .map(|cluster| {
                        let ref shaping_cluster = shaping_clusters[cluster.shaping_cluster_id];
                        let ref hb_buffer = hb_buffers[cluster.shaping_cluster_id];

                        if cluster.glyph_range.len() > 0 {
                            let glyph_positions = hb_buffer.as_ref().unwrap().glyph_positions();

                            let cluster_width = glyph_positions[cluster.glyph_range.clone()]
                                .iter()
                                .map(|x| {
                                    if is_vertical {
                                        x.y_advance
                                    } else {
                                        x.x_advance
                                    }
                                })
                                .sum::<i32>();

                            // Multiply by the font size
                            let font_size = if let ClusterContents::Text(ref props) =
                                shaping_cluster.contents
                            {
                                props.size
                            } else {
                                unreachable!()
                            };

                            cluster_width as f64 * (1.0 / FONT_SCALE) * font_size
                        } else {
                            match shaping_cluster.contents {
                                ClusterContents::Text(_) => 0.0,
                                ClusterContents::Foreign(x, _) => {
                                    let extents = x.extents();
                                    if is_vertical {
                                        extents[1]
                                    } else {
                                        extents[0]
                                    }
                                }
                            }
                        }
                    })
                    .sum();

                fn starts_with_space(s: &str) -> bool {
                    let bytes = s.as_bytes();
                    bytes.len() >= 1 && bytes[0] == 0x20
                        || bytes.len() >= 2 && bytes[0] == 0xc2 && bytes[1] == 0xa0
                }

                let is_glyph_info_expansible = |x: &harfbuzz_sys::hb_glyph_info_t| {
                    starts_with_space(&flattened[x.cluster as usize..])
                };

                let expansion;

                let try_justification = match (line.hard_break, para_style.text_align) {
                    (_, TextAlign::JustifyAll) | (false, TextAlign::Justify) => boundary.is_some(),
                    _ => false,
                };
                if try_justification {
                    // Perform justification. First, find the number of the
                    // expansion points.
                    let num_flex_points: usize = clusters
                        .iter()
                        .map(|cluster| {
                            let ref hb_buffer = hb_buffers[cluster.shaping_cluster_id];

                            if cluster.glyph_range.len() > 0 {
                                let glyph_infos = hb_buffer.as_ref().unwrap().glyph_infos();

                                glyph_infos[cluster.glyph_range.clone()]
                                    .iter()
                                    .filter(|x| is_glyph_info_expansible(*x))
                                    .count()
                            } else {
                                0
                            }
                        })
                        .sum();

                    if num_flex_points == 0 {
                        // Justifciation is impossible because there are no
                        // expansion points.
                        expansion = None;
                    } else {
                        // We got this
                        let container_width = line.x_coord_range.end - line.x_coord_range.start;
                        expansion = Some((container_width - line_width) / num_flex_points as f64);
                        line_width = container_width;
                    }
                } else {
                    expansion = None;
                }

                match (default_bidi_level.is_rtl(), para_style.text_align) {
                    (false, TextAlign::Start)
                    | (false, TextAlign::Justify)
                    | (false, TextAlign::JustifyAll)
                    | (true, TextAlign::End) => {
                        // Left-aligned
                        offs[0] = line.x_coord_range.start;
                        offs[1] = line.y_coord;
                    }
                    (false, TextAlign::End)
                    | (true, TextAlign::Start)
                    | (true, TextAlign::Justify)
                    | (true, TextAlign::JustifyAll) => {
                        // Right-aligned
                        offs[0] = line.x_coord_range.end - line_width;
                        offs[1] = line.y_coord;
                    }
                    (_, TextAlign::Center) => {
                        // Center-aligned
                        offs[0] =
                            line.x_coord_range.start + (line.x_coord_range.end - line_width) * 0.5;
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

                        // Prepare to retrieve a cursor into the original text
                        let mut reel = Reel::new(text, shaping_cluster.start_text);

                        // Multiply by the font size
                        let props =
                            if let ClusterContents::Text(ref props) = shaping_cluster.contents {
                                props
                            } else {
                                unreachable!()
                            };
                        let face_id = props.face_id.unwrap();
                        let face = self.font_face(face_id);
                        let scale = props.size * (1.0 / FONT_SCALE);

                        for (info, pos) in glyph_infos[range.clone()]
                            .iter()
                            .zip(glyph_positions[range.clone()].iter())
                        {
                            // Find out the glyph's style. `cluster` = absolute byte offset in `flattened`
                            let offset_flattened =
                                (info.cluster as usize - shaping_cluster.start_flattened) as isize;
                            // A byte offset inside a text of `flattened` directly translates to that of `text`
                            let cursor = reel.get(offset_flattened);
                            let char_style = text.attribute_at(cursor);

                            let char_style = para_style
                                .char_style
                                .as_char_style_ref()
                                .override_with(&char_style.as_char_style_ref());
                            let color = char_style.color;

                            // Get the extents of the glyph for bounds calculation
                            let hb_glyph_extents = face.hb_font.glyph_extents(info.codepoint);
                            // FIXME: `glyph_extents` returns `None` for some fonts even if the glyph
                            //        is not empty

                            output_glyphs.push(GlyphLayout {
                                position: Point2::new(
                                    offs[0] + pos.x_offset as f64 * scale,
                                    offs[1] + pos.y_offset as f64 * scale,
                                ),
                                scale,
                                face_id,
                                glyph_id: info.codepoint,
                                glyph_extents: hb_glyph_extents
                                    .as_ref()
                                    .map(GlyphExtents::from_hb_glyph_extents),
                                color,
                            });

                            offs[0] += pos.x_advance as f64 * scale;
                            offs[1] += pos.y_advance as f64 * scale;

                            if let Some(expansion) = expansion {
                                if is_glyph_info_expansible(info) {
                                    if is_vertical {
                                        offs[1] += expansion;
                                    } else {
                                        offs[0] += expansion;
                                    }
                                }
                            }
                        }
                    } else {
                        match shaping_cluster.contents {
                            ClusterContents::Text(_) => {}
                            ClusterContents::Foreign(x, _) => {
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

                LineLayout {}
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
    let needs_owned = text
        .iter()
        .enumerate()
        .any(|(i, x)| i > 0 || x.0.as_element_ref().foreign().is_some());

    if needs_owned {
        use itertools::Itertools;
        let concatenated = text
            .iter()
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

/// An utility type for efficient navigation of `Text` around a provided origin
/// point.
#[derive(Debug)]
struct Reel<'a, S: 'a, A: 'a> {
    text: &'a Text<S, A>,
    last: (Cursor, isize),
}

impl<'a, S: 'a + Span, A: 'a> Reel<'a, S, A> {
    fn new(text: &'a Text<S, A>, origin: Cursor) -> Self {
        Self {
            text,
            last: (origin, 0),
        }
    }

    /// Get a `Cursor` using a offset relative to `origin`.
    fn get(&mut self, offset: isize) -> Cursor {
        let (last_cursor, last_offset) = self.last;
        self.last = (
            self.text.offset(last_cursor, offset - last_offset).unwrap(),
            offset,
        );
        self.last.0
    }
}

#[derive(Debug, Clone)]
pub struct TextLayout {
    lines: Vec<LineLayout>,
    pub(crate) glyphs: Vec<GlyphLayout>,
}

impl TextLayout {
    /// Compute the visual bounding rectangle of the layouted text.
    pub fn visual_bounds(&self) -> [Point2<f64>; 2] {
        use std::f64::{INFINITY, NEG_INFINITY};
        self.glyphs.iter().fold(
            [
                Point2::new(INFINITY, INFINITY),
                Point2::new(NEG_INFINITY, NEG_INFINITY),
            ],
            |ret, glyph| {
                if let Some(bounds) = glyph.bounds() {
                    [
                        Point2::new(ret[0].x.min(bounds[0].x), ret[0].y.min(bounds[0].y)),
                        Point2::new(ret[1].x.max(bounds[1].x), ret[1].y.max(bounds[1].y)),
                    ]
                } else {
                    ret
                }
            },
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct LineLayout {
    // nothing so far
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GlyphLayout {
    pub position: Point2<f64>,
    pub scale: f64,
    glyph_extents: Option<GlyphExtents>,
    pub face_id: FontFaceId,
    pub glyph_id: u32,
    pub color: Option<RGBA<f32>>,
}

impl GlyphLayout {
    fn bounds(&self) -> Option<[Point2<f64>; 2]> {
        self.glyph_extents.map(|e| {
            let origin = self.position + e.origin.cast::<f64>().unwrap() * self.scale;
            [origin, origin + e.size.cast::<f64>().unwrap() * self.scale]
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct GlyphExtents {
    origin: Vector2<i32>,
    size: Vector2<i32>,
}

impl GlyphExtents {
    fn from_hb_glyph_extents(x: &harfbuzz_sys::hb_glyph_extents_t) -> Self {
        Self {
            origin: Vector2::new(x.x_bearing, -x.y_bearing),
            size: Vector2::new(x.width, -x.height),
        }
    }
}
