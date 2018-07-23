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
//! # Debugging
//!
//! Setting labels is supported by the following objects: `ArgPoolBuilder`,
//! `CmdBuffer`, `CmdQueueBuilder`, `BufferBuilder`, `ImageBuilder`,
//! `HeapBuilder`, `RenderPipelineBuilder`, `ComputePipelineBuilder`,
//! `SamplerBuilder`, and `LibraryBuilder`.
//! Labels are visible via a debugger interface, for example, Xcode's GPU Frame
//! Capture.
//!
//! All command encoders support debug commands (`begin_debug_group`,
//! `end_debug_group`, and `insert_debug_marker`). They are mapped to
//! `MTLCommandEncoder`'s methods and they are visible via Xcode's GPU Frame
//! Capture.
//!
//! # Limitations
//!
//! ## Implementation Limits
//!
//! - The upper bound of the number of vertex buffer bindings is `16`.
//!
//! ## Shaders
//!
//! - SPIRV-Cross does not adhere to the array base alignment rule as defined by
//!   the standard uniform buffer layout yet. It is advised that you only use
//!   16-byte aligned types (e.g., `vec4` or structs containing one) as element
//!   types for arrays defined in uniform buffers.
//!
#![feature(rust_2018_preview)]
#![warn(rust_2018_idioms)]
extern crate arrayvec;
extern crate atomic_refcell;
extern crate block;
extern crate cocoa;
extern crate iterpool;
extern crate ngsenumflags;
extern crate parking_lot;
extern crate refeq;
extern crate rspirv;
extern crate spirv_headers;
extern crate tokenlock;
extern crate xalloc;
extern crate zangfx_base;
extern crate zangfx_common;
pub extern crate zangfx_metal_rs;
extern crate zangfx_spirv_cross;

pub use zangfx_metal_rs as metal;

pub mod arg;
pub mod buffer;
pub mod cmd;
pub mod computepipeline;
pub mod device;
pub mod formats;
pub mod heap;
pub mod image;
pub mod limits;
pub mod renderpass;
pub mod renderpipeline;
pub mod sampler;
pub mod shader;
mod utils;

use std::fmt::Debug;

pub const MEMORY_REGION_GLOBAL: zangfx_base::MemoryRegionIndex = 0;

pub const MEMORY_TYPE_PRIVATE: zangfx_base::MemoryType = 0;
pub const MEMORY_TYPE_SHARED: zangfx_base::MemoryType = 1;

pub const MEMORY_TYPE_ALL_BITS: u32 = 0b11;

pub const QUEUE_FAMILY_UNIVERSAL: zangfx_base::QueueFamily = 0;

pub const MAX_NUM_VERTEX_BUFFERS: usize = 16;

/// The memory alignment requirement for uniform buffers.
pub const UNIFORM_BUFFER_MIN_ALIGN: zangfx_base::DeviceSize = 256;

/// The memory alignment requirement for storage buffers.
pub const STORAGE_BUFFER_MIN_ALIGN: zangfx_base::DeviceSize = 16;
