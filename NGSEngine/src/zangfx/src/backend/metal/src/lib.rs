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
//! # Performance Quirks
//!
//! Due to Metal's restrictions, `Heap` uses an emulated implementation for a
//! heap placed in a shared memory that does not use `MTLHeap`. As a result,
//! `use_heap` runs much slower for such heaps. In general, you should avoid
//! referencing shared resources through argument tables since their performance
//! is lower than private resources.
//!
//! # Debugging
//!
//! Setting labels is supported by the following objects: `ArgPoolBuilder`,
//! `CmdBuffer`, `CmdQueueBuilder`, `BufferBuilder`, `HeapBuilder`,
//! `ComputePipelineBuilder`, `SamplerBuilder`, and `LibraryBuilder`. Labels
//! are visible via a debugger interface, for example, Xcode's GPU Frame
//! Capture.
//!
//! All command encoders support debug commands (`begin_debug_group`,
//! `end_debug_group`, and `insert_debug_marker`). They are mapped to
//! `MTLCommandEncoder`'s methods and they are visible via Xcode's GPU Frame
//! Capture.
//!
extern crate block;
extern crate cocoa;
extern crate iterpool;
#[macro_use(flags)]
extern crate ngsenumflags;
extern crate parking_lot;
extern crate rspirv;
extern crate smallvec;
extern crate spirv_headers;
extern crate tokenlock;
extern crate xalloc;
extern crate refeq;
#[macro_use]
extern crate zangfx_base as base;
extern crate zangfx_common as common;
extern crate zangfx_metal_rs as metal;
extern crate zangfx_spirv_cross as spirv_cross;

pub mod arg;
pub mod buffer;
pub mod cmd;
pub mod device;
pub mod formats;
pub mod heap;
pub mod limits;
pub mod pipeline;
pub mod renderpass;
pub mod sampler;
pub mod shader;
mod utils;

use std::fmt::Debug;

pub static MEMORY_REGION_GLOBAL: base::MemoryRegionIndex = 0;

pub static MEMORY_TYPE_PRIVATE: base::MemoryType = 0;
pub static MEMORY_TYPE_SHARED: base::MemoryType = 1;

pub static MEMORY_TYPE_ALL_BITS: u32 = 0b11;

pub static QUEUE_FAMILY_UNIVERSAL: base::QueueFamily = 0;
