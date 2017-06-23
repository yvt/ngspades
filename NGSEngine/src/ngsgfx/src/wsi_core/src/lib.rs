//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX WSI Core
//! ===============
//!
//! Provides a common interface to the target window system.
//!

extern crate ngsgfx_core as core;
pub extern crate winit;
extern crate cgmath;

use std::sync::Arc;
use std::ops::Deref;
use std::fmt::Debug;

use core::Backend;
use cgmath::Vector2;

/// Window.
pub trait Window: Debug {
    type Backend: core::Backend;
    type CreationError: Debug;

    fn events_loop(&self) -> &winit::EventsLoop;
    fn winit_window(&self) -> &winit::Window;
    fn device(&self) -> &Arc<<Self::Backend as Backend>::Device>;
    fn acquire_framebuffer(&self) -> <Self::Backend as Backend>::ImageView;

    /// Inserts commands into the command buffer to present the rendered image.
    fn finalize_commands(&self, buffer: &mut <Self::Backend as Backend>::CommandBuffer);
    fn swap_buffers(&self);
    fn framebuffer_size(&self) -> Vector2<u32>;
    fn set_framebuffer_size(&self, size: Vector2<u32>);
}

/// Window with a constructor function.
pub trait NewWindow<T: Deref<Target = winit::EventsLoop>>: Window {
    fn new(
        wb: winit::WindowBuilder,
        events_loop: T,
        format: core::ImageFormat,
    ) -> Result<Self, Self::CreationError>
    where
        Self: Sized;
}
