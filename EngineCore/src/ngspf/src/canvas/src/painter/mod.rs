//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Draws graphical contents using a software-based renderer.
use crate::text::{FontConfig, TextLayout};
use cgmath::Vector2;
use rgb::RGBA;

/// An abstract interface used to issue draw operations.
pub trait Painter: crate::Debug {
    /// Save the current drawing state to the stack.
    ///
    /// The drawing state stored to the stack includes:
    ///
    ///  - The current transformation matrix.
    ///
    fn save(&mut self);

    /// Restore a drawing state from the stack.
    ///
    /// Triggers a panic if the stack is empty.
    fn restore(&mut self);

    /// Translate the current transformation matrix by the specified amount.
    fn translate(&mut self, x: Vector2<f64>);

    /// Scale the current transformation matrix by the specified factor.
    fn nonuniform_scale(&mut self, x: f64, y: f64);

    /// Set the current fill style to solid color, and use a given color for
    /// drawing.
    fn set_fill_color(&mut self, color: RGBA<f32>);

    /// Draw a given `TextLayout`.
    ///
    /// You must also supply the `FontConfig` from which the `TextLayout` was
    /// created. Specifying a wrong `FontConfig` may result in corrupted
    /// rendering or a panic.
    ///
    /// The text color is determined from the current fill style. This can be
    /// overridden on a by-character basis by the character style
    /// (`CharStyle::color`).
    fn fill_text_layout(&mut self, layout: &TextLayout, config: &FontConfig);
}

pub trait PainterUtils: Painter {
    /// Scale the current transformation matrix by the specified factor.
    fn scale(&mut self, x: f64) {
        self.nonuniform_scale(x, x)
    }
}

impl<T: Painter + ?Sized> PainterUtils for T {}

mod blend;
mod image;
mod text;
pub use self::image::*;
