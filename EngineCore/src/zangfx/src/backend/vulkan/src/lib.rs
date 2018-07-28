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
//! # Limitations
//!
//!  - The number of argument tables per root signature is limited to 32
//!    (`MAX_NUM_ARG_TABLES`).
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
pub mod sampler;
pub mod shader;
mod utils;

use std::fmt::Debug;
use std::ptr::{null, null_mut};
pub type AshDevice = ash::Device<ash::version::V1_0>;

pub use crate::utils::translate_generic_error;

pub const MAX_NUM_ARG_TABLES: usize = 32;
