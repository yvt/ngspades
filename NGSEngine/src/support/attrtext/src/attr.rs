//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ngsenumflags::BitFlags;
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

/// A set of character styles.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CharStyle {
    pub font_family: String,
    pub font_weight: Option<u16>,
    pub font_style: Option<FontStyle>,
    pub text_decoration: Option<TextDecorationFlags>,
    pub font_size: Option<f32>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, NgsEnumFlags)]
#[repr(u8)]
pub enum TextDecoration {
    Underline = 0b001,
    Overline = 0b010,
    Strikethrough = 0b100,
}

pub type TextDecorationFlags = BitFlags<TextDecoration>;
