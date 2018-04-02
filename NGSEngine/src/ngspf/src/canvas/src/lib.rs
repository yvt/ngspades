//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Manipulates 2D raster image data. This crate is a part of the
//! [NgsPF](../ngspf/index.html).
extern crate cgmath;
extern crate refeq;

extern crate ngspf_core as core;

mod image;

pub use self::image::*;
