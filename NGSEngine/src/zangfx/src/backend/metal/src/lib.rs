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
extern crate zangfx_common as common;
#[macro_use(zangfx_impl_object)]
extern crate zangfx_base as base;
extern crate ngsgfx_metal_rs as metal;
#[macro_use(flags)]
extern crate ngsenumflags;
extern crate block;
extern crate spirv_cross;
extern crate rspirv;
extern crate spirv_headers;
extern crate cocoa;

// TODO

pub mod formats;
pub mod limits;

pub static MEMORY_REGION_GLOBAL: base::MemoryRegionIndex = 0;

pub static MEMORY_TYPE_PRIVATE: base::MemoryType = 0;
pub static MEMORY_TYPE_SHARED: base::MemoryType = 1;

pub static QUEUE_FAMILY_UNIVERSAL: base::QueueFamily = 0;
