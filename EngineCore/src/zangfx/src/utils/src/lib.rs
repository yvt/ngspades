//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! [ZanGFX](../zangfx/index.html) utility library.
#![warn(rust_2018_idioms)]
#![feature(never_type)]

mod buffer;
pub mod cbstatetracker;
mod device;
pub mod uploader;
mod uploaderutils;

pub use crate::buffer::*;
#[doc(no_inline)]
pub use crate::cbstatetracker::*;
pub use crate::device::*;

/// ZanGFX Utils prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::{DeviceUtils, BufferUtils};
}
