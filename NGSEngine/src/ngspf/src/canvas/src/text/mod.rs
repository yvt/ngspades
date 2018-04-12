//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsPF Canvas “Featherweight” text layout engine.
//!
//! Backed by HarfBuzz and FreeType, performs text layouting including
//! character-to-glyph conversion, font shaping, and automatic font substitution.
//!
//! # Font Fallback
//!
//! Featherweight supports font fallback by trying every font registered to
//! `FontConfig`. However, if no matching font was found, it simply skips the
//! rendering of the unsupported text. Therefore, it's recommended that you
//! provide at least one font that covers entire the Unicode range, such as
//! [the Last Resort font].
//!
//! [the Last Resort font]: http://www.unicode.org/policies/lastresortfont_eula.html
//!
mod config;
mod font;
mod layout;
mod model;
pub use self::config::*;
pub use self::font::*;
pub use self::layout::TextLayout;
pub use self::model::*;

// Utilities
pub(crate) mod ftutils;
mod hbutils;
mod wordwrap;

/// The constant font scale value for internally stored font objects.
const FONT_SCALE: f64 = 65536.0;
