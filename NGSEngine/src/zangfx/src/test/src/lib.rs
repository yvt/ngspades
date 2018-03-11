//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Test framework for ZanGFX implementations.
#![feature(test)]
#![macro_use]
extern crate zangfx_base as gfx;
extern crate zangfx_common as common;
#[macro_use]
extern crate ngsenumflags;
#[macro_use]
extern crate include_data;
#[doc(hidden)]
pub extern crate test;

pub mod backend_benches;
pub mod backend_tests;
pub mod utils;
