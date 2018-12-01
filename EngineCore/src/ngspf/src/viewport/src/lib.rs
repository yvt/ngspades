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
pub use {zangfx, rgb};

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
pub use self::layer::*;
pub use self::port::*;
pub use self::window::*;
pub use self::workspace::*;

mod gfxutils;

#[cfg(target_os = "macos")]
mod metalutils;

use std::fmt::{self, Debug};
#[allow(unused_imports)]
use std::ptr::{null, null_mut};
