//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! [ZanGFX](../zangfx/index.html) utility library.
extern crate itertools;
extern crate zangfx_base as base;
extern crate zangfx_common as common;
#[macro_use]
extern crate ngsenumflags;

pub mod cbstatetracker;
mod device;
pub mod uploader;
mod uploaderutils;

#[doc(no_inline)]
pub use cbstatetracker::*;
pub use device::DeviceUtils;

/// ZanGFX Utils prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use DeviceUtils;
}
