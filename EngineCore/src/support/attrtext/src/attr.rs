//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use rgb::RGBA16;

/// Sets of partially specified styling properties that can be constructed by
/// overriding the properties of one with those of another.
pub trait Override<Rhs = Self> {
    /// Override styling properties using another ones, constructing a new set
    /// of styling properties.
    fn override_with(&self, x: &Rhs) -> Self;
}

impl<T: Clone> Override<()> for T {
    fn override_with(&self, _: &()) -> Self {
        self.clone()
    }
}

impl<T: Clone> Override for Option<T> {
    fn override_with(&self, x: &Option<T>) -> Self {
        x.as_ref().or_else(|| self.as_ref()).cloned()
    }
}

/// A set of common character styles. Largely based on CSS (the *Cascading Style
/// Sheets* language).
///
/// Although we provide rough meanings of these properties as a guideline, the
/// precise definitions of them are up to the application.
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
    pub font_size: Option<f32>,
    /// The color of the text.
    pub color: Option<RGBA16>,
}

impl Override for CharStyle {
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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

bitflags! {
    pub struct TextDecorationFlags: u8 {
        const UNDERLINE = 0b001;
        const OVERLINE = 0b010;
        const STRIKETHROUGH = 0b100;
    }
}
