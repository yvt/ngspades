//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGameGFX — Nightingales Game GFX
#![feature(unsized_locals)] // For calling boxed `FnOnce`

mod asyncuploader;
pub mod config;
mod di;
mod passman;
pub mod port;
mod utils;

pub use crate::utils::progress::Progress;
