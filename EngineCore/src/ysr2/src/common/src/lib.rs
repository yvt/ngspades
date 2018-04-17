//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(platform_intrinsics, cfg_target_feature)]

#[cfg(feature = "xdispatch")]
extern crate xdispatch;
#[cfg(feature = "xdispatch")]
extern crate num_cpus;
extern crate cgmath;
extern crate simd;
extern crate arrayvec;
extern crate parking_lot;

pub mod dispatch;
pub mod nodes;
mod simdutils;
pub mod slicezip;
pub mod stream;
pub mod utils;
pub mod values;
