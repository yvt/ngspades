//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! ZanGFX Metal Backend — Implements a ZanGFX backend using Apple's Metal 2 API.
//!
//! Metal is one of the primary target APIs of ZanGFX as well as its
//! predecessor, NgsGFX. For this reason, ZanGFX is designed to run efficiently
//! on Metal.
//!
//! # Implementation Details
//!
//! ## Ownership of raw pointers
//!
//! Metal objects require manual reference counting using `[NSObject retain]`
//! and `[NSObject release]`. When dealing with ref-counted objects, it is
//! crucial to maintain the ownership properly. In general, this crate follows
//! the pattern shown below:
//!
//!  - Methods named `new` increases the reference count when receiving an
//!    object, thus creating a new strong reference.
//!  - Conversely, methods named `from_raw` do not increase the reference count.
//!  - No method increases the reference count when returning an object.
//!
extern crate block;
extern crate cocoa;
#[macro_use(flags)]
extern crate ngsenumflags;
extern crate parking_lot;
extern crate rspirv;
extern crate spirv_headers;
extern crate tokenlock;
extern crate iterpool;
extern crate xalloc;
extern crate smallvec;
#[macro_use]
extern crate zangfx_base as base;
extern crate zangfx_common as common;
extern crate zangfx_metal_rs as metal;
extern crate zangfx_spirv_cross as spirv_cross;

// TODO

pub mod arg;
pub mod buffer;
pub mod cmd;
pub mod device;
pub mod formats;
pub mod heap;
pub mod limits;
pub mod sampler;
pub mod shader;
mod utils;

use std::fmt::Debug;

pub static MEMORY_REGION_GLOBAL: base::MemoryRegionIndex = 0;

pub static MEMORY_TYPE_PRIVATE: base::MemoryType = 0;
pub static MEMORY_TYPE_SHARED: base::MemoryType = 1;

pub static QUEUE_FAMILY_UNIVERSAL: base::QueueFamily = 0;
