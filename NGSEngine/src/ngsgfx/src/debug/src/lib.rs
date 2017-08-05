//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Debug Tools
//! ==================
//!
//! Provides debug tools for NgsGFX.
//!
//! This crate is re-exported by the `ngsgfx` main crate as `::debug`.
//!
extern crate ngsgfx_core as core;
extern crate term;
extern crate chrono;

pub mod report;
