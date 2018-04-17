//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[macro_use]
extern crate ngscom;
#[macro_use]
extern crate lazy_static;
extern crate ngsbase;
extern crate cgmath;

mod entry;

pub use self::entry::ngsengine_create;
