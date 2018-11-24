//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! [NgsPF](../ngspf/index.html) Viewport API.
//!
//! # Measurement Units
//!
//! All distances are measured in points / CSS pixels / device independent
//! pixels, with the exceptions of the cases where the exact number of physical
//! pixels is important.
//!
extern crate cggeom;
extern crate cgmath;
extern crate iterpool;
extern crate refeq;
pub extern crate rgb;
extern crate winit;

#[macro_use(flags)]
extern crate flags_macro;

extern crate ngsenumflags;
#[macro_use]
extern crate ngsenumflags_derive;

extern crate ngspf_canvas as canvas;
extern crate ngspf_core as core;
pub extern crate zangfx;

#[macro_use]
extern crate include_data;

#[cfg(target_os = "macos")]
extern crate cocoa;
#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

extern crate xdispatch;

mod compositor;
// mod device;
mod imagemanager;
mod layer;
mod port;
mod portrender;
mod temprespool;
mod window;
mod workspace;
mod wsi;

// pub use device::*;
pub use layer::*;
pub use port::*;
pub use window::*;
pub use workspace::*;

mod gfxutils;

#[cfg(target_os = "macos")]
mod metalutils;

use std::fmt::{self, Debug};
#[allow(unused_imports)]
use std::ptr::{null, null_mut};
