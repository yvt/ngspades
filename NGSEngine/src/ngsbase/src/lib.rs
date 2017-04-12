//! Nightingales Base / Interop
//! =============================
//!
//! This crate includes basic data types and definitions of COM interfaces.

//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[macro_use]
extern crate ngscom;

mod interop;
pub use interop::*;
