//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Metal Backend
//! ====================
//!
//! Feature Mappings
//! ----------------
//!
//!  - **Heap** - A manually managed heap is available as `MTLHeap`, but it is not supported
//!    on macOS yet. Since we are targetting desktop platforms, no implementations that make
//!    use of `MTLHeap` are provided.
//!
//!    `MTLBuffer` provides a method that allows users to sub-allocate its portion to create
//!    texture views, but this is inappropriate for a general use since it is not designed for
//!    performance but rather a way to create linearly layouted textures.
//!  - **Semaphore** - `MTLFence` would be the nearest counterpart, but it is not supported on macOS
//!    where hazards are handled automatically.
//!  - **Fence** - Mapped to callbacks from `MTLCommandBuffer`.
//!  - **BufferView** - No direct Metal counterparts are available. TODO: emulate by 2D texture?
//!
extern crate ngsgfx_core as core;

mod sync;

/// Defines implementations for NgsGFX primitives.
pub mod imp {
    pub use super::sync::*;
}
