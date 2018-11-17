//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGameGFX — Nightingales Game GFX
#![feature(unsized_locals)] // For calling boxed `FnOnce`
#![feature(futures_api)]
#![feature(pin)]
#![feature(arbitrary_self_types)]

mod asyncuploader;
mod cbtasks;
pub mod config;
mod di;
mod passman;
pub mod port;
mod staticdata;
mod taskman;
mod utils;

pub use crate::utils::progress::Progress;
