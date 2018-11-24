//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a string type with character styling attributes (bold, color, ...).
extern crate rgb;
extern crate itertools;

#[macro_use]
extern crate bitflags;

extern crate opaque_typedef;
#[macro_use]
extern crate opaque_typedef_macros;

pub mod attr;
pub mod text;
mod macros;

#[doc(no_inline)]
pub use attr::*;

#[doc(no_inline)]
pub use text::{Span, Text, TextBuf};
