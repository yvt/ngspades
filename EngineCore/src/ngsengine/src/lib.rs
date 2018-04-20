//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

extern crate cgmath;
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate ngscom;
extern crate ngsbase;
extern crate ngspf;
extern crate ngspf_com;

mod entry;

pub use self::entry::ngsengine_create;
