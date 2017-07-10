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
use std::fmt::Debug;

use core::{Environment, Backend};
use cgmath::Vector2;

/// Window.
pub trait Window: Debug {
    type Backend: core::Backend;

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
pub trait NewWindow: Window + Sized {
    type Environment: core::Environment<Backend = Self::Backend>;
    type CreationError: Debug;

    fn new(
        wb: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        instance: &<Self::Environment as Environment>::Instance,
        format: core::ImageFormat,
    ) -> Result<Self, Self::CreationError>;

    /// Updates the supplied `InstanceBuilder` to meet the requirements of this WSI backend.
    #[allow(unused_variables)]
    fn modify_instance_builder(builder: &mut <Self::Environment as Environment>::InstanceBuilder) {}
}
