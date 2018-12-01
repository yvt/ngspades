//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Manipulates 2D raster image data. This crate is a part of the
//! [NgsPF](../ngspf/index.html).
#![feature(never_type)]
pub use cgmath;

use std::fmt::Debug;
use std::ops::Deref;
use std::ptr::{null, null_mut};

mod image;
pub mod painter;
pub mod text;
mod transform2d;

pub use self::image::*;
pub use self::transform2d::*;

// Utilities
mod iterutils;

pub mod prelude {
    #[doc(no_inline)]
    pub use crate::painter::PainterUtils;
}
