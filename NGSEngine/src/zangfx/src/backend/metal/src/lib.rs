//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Metal Backend — Implements a ZanGFX backend using Apple's Metal 2 API.
//!
//! Metal is one of the primary target APIs of ZanGFX as well as its
//! predecessor, NgsGFX. For this reason, ZanGFX is designed to run efficiently
//! on Metal.
extern crate zangfx_common as common;
extern crate zangfx_base as base;
// TODO