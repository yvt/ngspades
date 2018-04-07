//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Performs text layouting including character-to-glyph conversion,
//! font shaping, and automatic font substitution.
//!
//! # Technical details
//!
//! ## Terminology
//!
//! **Character styles** are properties associated with each code point that
//! control their appearance, including their color, decorative elements (e.g.,
//! underline), and the font used to render the text.
//!
//! **Font styles** are a subset of character styles that affect the choice of
//! the font face used to render the text. Ligatures and font shaping cannot
//! occur accross the font style boundaries.
//!
//! ## Text layouting
//!
//! The text layouting proceeds as the following:
//!
//!  1. The application provides a string representing the input paragraph. The
//!     input string can be either a plain text (`String`) or an attributed text
//!     (`attrtext::Text`). The application also provides a `ParagraphStyle` and
//!     `Boundary`.
//!
//!  2. The final font style is computed for each character (precisely, single
//!     code point or TODO: multiple code points to support variation sequences).
//!     The BiDi level of each byte in the text is analyzed by [`unicode-bidi`].
//!     The paragraph embedding level is determined from
//!     `ParagraphStyle::direction`.
//!     Finally, the text is broken into a series of *shaping clusters*, each of
//!     which contains a sequence of characters sharing an identical set of
//!     font styles and the same BiDi run direction.
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
//!

mod config;
pub use self::config::*;
