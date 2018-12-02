//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! This crate is a part of [NgsGameGFX](../ngsgamegfx/index.html).
//!
//! Provides components for facilitating run-time task graph construction.
#![feature(unsized_locals)] // For calling boxed `FnOnce`

pub mod cbtasks;
pub mod passman;
pub mod taskman;