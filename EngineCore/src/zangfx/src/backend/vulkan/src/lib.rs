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
//! # Inter-queue operations
//!
//! This backend supports inter-queue operations.
//!
//! # Backend-specific behaviors
//!
//! *Default queue*: The base interface specifies that how the default value of
//! `*Builder::queue` is determined is backend-dependent. In this backend,
//! `Device` maintains the default queue to be used during object creation. The
//! first created `CmdQueue` from it will be used as the default unless it is
//! explicitly specified via [`crate::device::Device::set_default_queue`]. If
//! there is no default value set at the point when it is required (i.e., when
//! a builder's `build` is called), a dummy value that is not associated with
//! any queue will be used instead.
//!
//! # Unsafety
//!
//! This backend implementation is known to cause an undefined behavior
//! (including compromising the memory safety and integrity otherwise provided
//! by Rust), mainly due to an incorrect usage of the Vulkan API, in the
//! situations including (but not limited to) the following:
//!
//!  - Using objects that originate from a different `VkDevice`.
//!  - Uses of uninitialized argument tables.
//!  - Uses of already destroyed argument tables.
//!  - Updating argument tables which are referenced by any pending command
//!    buffers.
//!  - Specifying parameters exceeding the hardware limitation indicated by
//!    `DeviceLimits`
//!  - Uses of invalid SPIRV code.
//!  - Out-of-bounds buffer accesses. The `robustBufferAccess` feature can be
//!    used to make them safe when buffers are accessed from shader code.
//!  - Out-of-bounds image accesses.
//!  - Doing queue ownership transfer operations improperly.
//!  - Not calling `use_resource_core` when you have to.
//!  - Simultaneous accesses to an image layer via a render target and a shader
//!    argument at the same time.
//!  - Incompatibility between a root signature and shader.
//!
//! ## Rust undefined behaviors
//!
//! # Limitations
//!
//!  - The number of argument tables per root signature is limited to 32
//!    (`MAX_NUM_ARG_TABLES`).
//!  - The number of referenced resources per command buffer is limited to
//!    around 4 billions.
//!
#![feature(unsize)]
#![feature(rust_2018_preview)]
#![warn(rust_2018_idioms)]

#[allow(rust_2018_idioms)]
pub extern crate ash;

pub mod arg;
pub mod buffer;
pub mod cmd;
pub mod device;
pub mod formats;
pub mod heap;
pub mod image;
pub mod limits;
pub mod pipeline;
pub mod renderpass;
mod resstate;
pub mod sampler;
pub mod shader;
mod utils;

use std::fmt::Debug;
use std::ptr::{null, null_mut};
pub type AshDevice = ash::Device<ash::version::V1_0>;

pub use crate::utils::translate_generic_error;

pub const MAX_NUM_ARG_TABLES: usize = 32;

/// The maximum number of command buffers (per queue) that can be active
/// simultaneously.
pub const MAX_NUM_ACTIVE_CMD_BUFFERS: usize = 16;
