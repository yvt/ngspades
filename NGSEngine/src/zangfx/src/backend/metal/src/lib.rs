//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Metal Backend — Implements a ZanGFX backend using Apple's Metal 2 API.
//!
//! Metal is one of the primary target APIs of ZanGFX as well as its
//! predecessor, NgsGFX. For this reason, ZanGFX is designed to run efficiently
//! on Metal.
extern crate block;
extern crate cocoa;
#[macro_use(flags)]
extern crate ngsenumflags;
extern crate parking_lot;
extern crate rspirv;
extern crate spirv_headers;
extern crate tokenlock;
#[macro_use]
extern crate zangfx_base as base;
extern crate zangfx_common as common;
extern crate zangfx_metal_rs as metal;
extern crate zangfx_spirv_cross as spirv_cross;

// TODO

pub mod cmd;
pub mod device;
pub mod formats;
pub mod limits;
pub mod sampler;
pub mod shader;
mod utils;

use std::fmt::Debug;

pub static MEMORY_REGION_GLOBAL: base::MemoryRegionIndex = 0;

pub static MEMORY_TYPE_PRIVATE: base::MemoryType = 0;
pub static MEMORY_TYPE_SHARED: base::MemoryType = 1;

pub static QUEUE_FAMILY_UNIVERSAL: base::QueueFamily = 0;
