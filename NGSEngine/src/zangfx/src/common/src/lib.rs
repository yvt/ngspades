//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! # ZanGFX Common: Utilities
#![feature(raw)]
#![feature(unsize)]
pub extern crate cgmath;

mod error;
mod smallbox;
pub use self::error::*;
pub use self::smallbox::*;
