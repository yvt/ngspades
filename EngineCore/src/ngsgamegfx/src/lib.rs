//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGameGFX — Nightingales Game GFX
//!
//! # Sub-crates
//!
//!  - [`ngsgamegfx_common`](../ngsgamegfx_common/index.html)
//!  - [`ngsgamegfx_graph`](../ngsgamegfx_graph/index.html)
//!
#![feature(arbitrary_self_types)]
#![feature(futures_api)]
#![feature(unsized_locals)] // For calling boxed `FnOnce`

mod asyncuploader;
pub mod config;
mod di;
pub mod port;
mod staticdata;

mod testpass; // TEST

pub use ngsgamegfx_common::progress::Progress;
