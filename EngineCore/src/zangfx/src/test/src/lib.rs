//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Test framework for ZanGFX implementations.
#![feature(rust_2018_preview)]
#![warn(rust_2018_idioms)]
#![feature(test)]

#[doc(hidden)]
pub extern crate test;

pub mod backend_benches;
pub mod backend_tests;
pub mod utils;
