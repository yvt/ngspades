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
use cgmath::Vector3;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SwapchainError {
    GenericError(core::GenericError),

    /// The state of swapchain has been changed by some external factors and the
    /// swapchain needs to be created again before next images to be presented.
    OutOfDate,

    /// The target of the swapchain is lost. The swapchain no longer can be created again on the same target.
    TargetLost,

    /// The next image did not became available within.a predetermined duration of time.
    NotReady,
}

#[derive(Debug, Clone)]
pub struct FrameDescription {
    /// The set of `DeviceEngine`s that will potentially wait on the `Drawable::acquiring_fence()`.
    pub acquiring_engines: core::DeviceEngineFlags,

    /// The set of `DeviceEngine`s that will potentially update the `Drawable::releasing_fence()`.
    pub releasing_engines: core::DeviceEngineFlags,
}

pub trait Drawable: Debug {
    type Backend: core::Backend;

    fn image(&self) -> &<Self::Backend as Backend>::Image;

    /// The `Fence` object that must be waited for before the `image` gets written
    /// with new contents.
    ///
    /// Do not use the returned `Fence` for other purposes!
    fn acquiring_fence(&self) -> Option<&<Self::Backend as Backend>::Fence> { None }

    /// The `Fence` object that must be updated after the `image` was updated with
    /// the contents to be presented.
    ///
    /// Do not use the returned `Fence` for other purposes!
    fn releasing_fence(&self) -> Option<&<Self::Backend as Backend>::Fence> { None }

    /// Inserts commands into the command buffer to prepare the presentation of the image.
    ///
    /// The command buffer must in the `Recording` state.
    /// There must be an active command pass, and the pass's engine must be the one
    /// having an ownership on the image.
    ///
    /// There must not be an active render subpass. This must be called after the
    /// last subpass in a render pass was ended.
    ///
    /// `stage` and `access` specify the pipeline stage and access type that were used
    /// to write the image, respectively.
    /// `layout` specifies the current image layout.
    fn finalize(
        &self,
        command_buffer: &mut <Self::Backend as Backend>::CommandBuffer,
        state: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        layout: core::ImageLayout,
    );

    /// Present the image.
    ///
    /// Must be called after the command buffer in which a presentation preparation command
    /// was a encoded by `finalize` was submitted.
    ///
    /// This also puts this drawable back to the swapchain for the future use, which means
    /// this must be called for every drawable acquired.
    fn present(&self);
}

#[derive(Debug, Clone)]
pub struct DrawableInfo {
    pub extents: Vector3<u32>,
    pub num_array_layers: u32,
    pub format: core::ImageFormat,
    pub colorspace: ColorSpace,
}

/// Swapchain.
///
/// An application must ensure all images acquired from it are no longer in use
/// by the application or device before dropping `Swapchain`.
pub trait Swapchain: Debug {
    type Backend: core::Backend;
    type Drawable: Drawable<Backend = Self::Backend>;

    fn device(&self) -> &<Self::Backend as Backend>::Device;

    /// Acquire a next `Drawable`.
    fn next_drawable(
        &self,
        description: &FrameDescription,
    ) -> Result<Self::Drawable, SwapchainError>;

    fn drawable_info(&self) -> DrawableInfo;
}

/// Window.
pub trait Window: Debug {
    type Backend: core::Backend;
    type Swapchain: Swapchain<Backend = Self::Backend>;

    fn winit_window(&self) -> &winit::Window;
    fn device(&self) -> &Arc<<Self::Backend as Backend>::Device>;

    fn swapchain(&self) -> &Self::Swapchain;

    /// Create a swapchain that matches the current state of the `Window`.
    ///
    /// For example, call this whenever the window size has changed.
    fn update_swapchain(&mut self);
}

/// Window with a constructor function.
pub trait NewWindow: Window + Sized {
    type Environment: core::Environment<Backend = Self::Backend>;
    type CreationError: Debug;

    fn new(
        wb: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        instance: &<Self::Environment as Environment>::Instance,
        swapchain_description: &SwapchainDescription,
    ) -> Result<Self, Self::CreationError>;

    /// Updates the supplied `InstanceBuilder` to meet the requirements of this WSI backend.
    #[allow(unused_variables)]
    fn modify_instance_builder(builder: &mut <Self::Environment as Environment>::InstanceBuilder) {}
}

#[derive(Debug, Clone)]
pub struct SwapchainDescription<'a> {
    pub desired_formats: &'a [(Option<core::ImageFormat>, Option<ColorSpace>)],

    /// Specifies the usage of the images returned by drawables.
    ///
    /// An excessive number of flags might inhibit optimizations.
    pub image_usage: core::ImageUsageFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    /// Color values are interpreted using the non-linear sRGB color space.
    SrgbNonlinear,
}
