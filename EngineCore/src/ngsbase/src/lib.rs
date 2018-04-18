//! Nightingales Base
//!
//! This crate includes basic data types and definitions of COM interfaces
//! automatically generated from the .NET assembly.
//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[macro_use]
extern crate ngscom;
extern crate cgmath;
extern crate cggeom;
extern crate ngsenumflags;
extern crate num_traits;
#[macro_use]
extern crate ngsenumflags_derive;
extern crate rgb;

mod interop;

pub use interop::*;
