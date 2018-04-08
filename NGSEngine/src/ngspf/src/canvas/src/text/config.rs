//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use attrtext;
use harfbuzz;
use rgb::RGBA;
use std::ops::Range;

// Reuse useful definitions
pub use attrtext::{FontStyle, TextDecoration, TextDecorationFlags};

/// A set of paragraph styles.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphStyle {
    pub line_height: LineHeight,
    pub direction: Direction,
    pub alignment: Alignment,

    pub word_wrap_mode: WordWrapMode,

    /// The default character style for this paragraph.
    ///
    /// The layout engine will first look at the character styles embedded in
    /// the supplied text. If some property values are missing, then it'll
    /// look at the values specified here. If the values cannot be found
    /// at this point yet, the layout engine will trigger a panic.
    ///
    /// `font_family` is optional.
    pub char_style: CharStyle,
}

impl ParagraphStyle {
    /// Construct a `ParagraphStyle` with reasonable default values suitable for
    /// rendering a English user-interface text.
    pub fn new() -> Self {
        Self {
            line_height: LineHeight {
                minimum: 0.0,
                factor: 1.0,
            },
            direction: Direction::LeftToRight,
            alignment: Alignment::Start,
            word_wrap_mode: WordWrapMode::MinNumLines,
            char_style: CharStyle::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WordWrapMode {
    /// Minimizes the number of lines. This mode is commonly used by most
    /// operating systems and word processors.
    MinNumLines,

    /// Minimizes the [raggedness]. This mode is often used by high-quality
    /// typesetting systems such as Adobe InDesign and LaTeX.
    ///
    /// [raggedness]: https://en.wikipedia.org/wiki/Line_wrap_and_word_wrap#Minimum_raggedness
    MinRaggedness,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineHeight {
    /// The minimum absolute value of the line height, measured in points.
    pub minimum: f64,

    /// The line height multiplier applied to the font size.
    pub factor: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    Start,
    End,
    Center,
    Justify,
    JustifyAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

impl Direction {
    pub fn is_vertical(&self) -> bool {
        match self {
            &Direction::LeftToRight => false,
            &Direction::RightToLeft => false,
            &Direction::TopToBottom => true,
            &Direction::BottomToTop => true,
        }
    }
}

/// Defines the boundary shape of the text container.
pub trait Boundary {
    /// Compute the available width for a specific line of a text.
    ///
    /// For horizontally rendered texts (i.e. uses `Direction::LeftToRight` or,
    /// `Direction::RightToLeft`), `line_position` specifies the range of the Y
    /// coordinate where the line is located. The return value is a range of
    /// the X coordinate where the text can be placed.
    ///
    /// For vertically rendered texts, the roles of the X and Y coordinates are
    /// swapped.
    fn line_range(&self, line_position: Range<f64>, line: usize) -> Option<Range<f64>>;
}

pub struct BoxBoundary {
    range: Range<f64>,
}

impl BoxBoundary {
    /// Construct a `BoxBoundary` with a constant X (or Y for vertically
    /// rendered texts) coordinate range.
    pub fn new(range: Range<f64>) -> Self {
        Self { range }
    }

    /// Construct a `BoxBoundary` with an infinitely large containing box.
    pub fn infinite() -> Self {
        Self::new(0.0..::std::f64::INFINITY)
    }
}

impl Boundary for BoxBoundary {
    fn line_range(&self, _: Range<f64>, _: usize) -> Option<Range<f64>> {
        Some(self.range.clone())
    }
}

/// A set of character styles. Largely based on CSS (the *Cascading Style
/// Sheets* language).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CharStyle {
    /// A list of font family names, sorted according to the priority.
    pub font_family: Vec<String>,
    /// The weight of the font.
    pub font_weight: Option<u16>,
    /// The style of the font.
    pub font_style: Option<FontStyle>,
    /// Specifies the appearance of decorative lines used on the text.
    pub text_decoration: Option<TextDecorationFlags>,
    /// The size of the font.
    pub font_size: Option<f64>,
    /// The color of the text.
    pub color: Option<RGBA<f64>>,
    /// The language of the text.
    pub language: Option<Language>,
    /// The script of the text.
    pub script: Option<Script>,
}

impl CharStyle {
    /// Construct a `CharStyle` with reasonable default values.
    ///
    /// The returned `CharStyle` has all properties with reasonable default
    /// values, with the exception of `font_family`, for which you have to
    /// provide a value anyway.
    ///
    /// Use `CharStyle::default()` instead if you want a `CharStyle` with all
    /// properties unspecified.
    pub fn new() -> Self {
        Self {
            font_family: Vec::new(),
            font_weight: Some(400),
            font_style: Some(FontStyle::Normal),
            text_decoration: Some(TextDecorationFlags::empty()),
            font_size: Some(16.0),
            color: Some(RGBA::new(0.0, 0.0, 0.0, 1.0)),
            language: Some(Default::default()),
            script: Some(Default::default()),
        }
    }
}

impl attrtext::Override for CharStyle {
    fn override_with(&self, x: &CharStyle) -> CharStyle {
        CharStyle {
            font_family: if x.font_family.is_empty() {
                self.font_family.clone()
            } else {
                x.font_family.clone()
            },
            font_weight: x.font_weight.or(self.font_weight),
            font_style: x.font_style.or(self.font_style),
            text_decoration: x.text_decoration.or(self.text_decoration),
            font_size: x.font_size.or(self.font_size),
            color: x.color.or(self.color),
            language: x.language.or(self.language),
            script: x.script.or(self.script),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct Language(harfbuzz::hb_language_t);

unsafe impl Sync for Language {}
unsafe impl Send for Language {}

impl Language {
    pub unsafe fn from_raw(x: harfbuzz::hb_language_t) -> Self {
        Language(x)
    }

    pub fn from_iso639(x: &str) -> Option<Self> {
        let x = unsafe {
            harfbuzz::RUST_hb_language_from_string(x.as_ptr() as *mut u8 as *mut _, x.len() as _)
        };
        use std::ptr::null_mut;
        if x == null_mut() {
            None
        } else {
            Some(Language(x))
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Language(unsafe { harfbuzz::RUST_hb_language_get_default() })
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct Script(harfbuzz::hb_script_t);

impl Script {
    pub unsafe fn from_raw(x: harfbuzz::hb_script_t) -> Self {
        Script(x)
    }

    pub fn from_iso15924(x: &str) -> Option<Self> {
        let x = unsafe {
            harfbuzz::RUST_hb_script_from_string(x.as_ptr() as *mut u8 as *mut _, x.len() as _)
        };
        if x == harfbuzz::HB_SCRIPT_INVALID {
            None
        } else {
            Some(Script(x))
        }
    }
}

impl Default for Script {
    /// Return `Script::from_raw(harfbuzz::HB_SCRIPT_COMMON)`.
    fn default() -> Self {
        Script(harfbuzz::HB_SCRIPT_COMMON)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        CharStyle::new();
        Language::default();
        Script::default();
    }
}
