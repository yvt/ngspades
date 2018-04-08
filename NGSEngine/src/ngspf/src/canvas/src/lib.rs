//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Manipulates 2D raster image data. This crate is a part of the
//! [NgsPF](../ngspf/index.html).
extern crate attrtext;
extern crate cgmath;
extern crate freetype;
extern crate harfbuzz;
extern crate refeq;
extern crate rgb;
#[macro_use]
extern crate lazy_static;

extern crate ngspf_core as core;

use std::fmt::Debug;
use std::ops::Deref;
use std::ptr::{null, null_mut};

mod image;
pub mod text;

pub use self::image::*;
