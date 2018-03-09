//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! ZanGFX Vulkan Backend — Implements a ZanGFX backend using the Vulkan API.
//!
//! Vulkan is one of the primary target APIs of ZanGFX as well as its
//! predecessor, NgsGFX. For this reason, ZanGFX is designed to run efficiently
//! on Vulkan.
//!
extern crate iterpool;
#[macro_use(flags)]
extern crate ngsenumflags;
extern crate parking_lot;
extern crate smallvec;
extern crate tokenlock;
extern crate xalloc;
extern crate refeq;
pub extern crate ash;
#[macro_use]
extern crate zangfx_base as base;
extern crate zangfx_common as common;

pub mod arg;
pub mod buffer;
pub mod cmd;
pub mod device;
pub mod formats;
pub mod heap;
pub mod image;
pub mod limits;
mod utils;

use std::fmt::{self, Debug};
use std::ops::Deref;
use std::ptr::{null, null_mut};
pub type AshDevice = ash::Device<ash::version::V1_0>;
