//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! This crate is a part of [NgsGameGFX](../ngsgamegfx/index.html).
//!
//! Provides utility types and function to be internally used by NgsGameGFX.
#![feature(futures_api)]
#![feature(pin)]
#![feature(arbitrary_self_types)]

pub mod any;
pub mod futures;
pub mod iterator_mut;
pub mod owning_ref;
pub mod progress;
