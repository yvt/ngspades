//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#[cfg(feature = "xdispatch")]
extern crate xdispatch;
#[cfg(feature = "xdispatch")]
extern crate num_cpus;
extern crate cgmath;

pub mod dispatch;
pub mod slicezip;
pub mod stream;
pub mod values;
