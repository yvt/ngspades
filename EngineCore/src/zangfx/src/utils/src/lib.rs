//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! [ZanGFX](../zangfx/index.html) utility library.
#![warn(rust_2018_idioms)]
#![feature(never_type)]

pub mod asyncheap;
mod buffer;
pub mod futuresapi;
pub mod cbstatetracker;
mod device;
pub mod streamer;
pub mod uploader;
mod uploaderutils;

pub use crate::buffer::*;
#[doc(no_inline)]
pub use crate::cbstatetracker::*;
#[doc(no_inline)]
pub use crate::futuresapi::*;
pub use crate::device::*;

/// ZanGFX Utils prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::{DeviceUtils, BufferUtils, CmdBufferFutureExt};
}
