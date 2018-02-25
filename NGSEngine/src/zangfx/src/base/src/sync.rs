//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for synchronization objects.
use std::any::Any;
use std::fmt::Debug;
use std::ops::Range;
use common::Result;
use handles::{Barrier, Buffer, Fence, Image};
use {AccessTypeFlags, DeviceSize};
use resources::{ImageLayout, ImageSubRange};

/// Trait for building fences.
///
/// # Valid Usage
///
///  - No instance of `FenceBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::sync::FenceBuilder;
///     # fn test(device: &Device) {
///     let image = device.build_fence()
///         .build()
///         .expect("Failed to create a fence.");
///     # }
///
pub trait FenceBuilder: Send + Sync + Any + Debug + AsRef<Any> + AsMut<Any> {
    /// Build a `Fence`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<Fence>;
}

/// Trait for building barriers.
///
/// # Valid Usage
///
///  - No instance of `BarrierBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::AccessType;
///     # use zangfx_base::sync::BarrierBuilder;
///     # fn test(device: &Device) {
///     let image = device.build_barrier()
///         .global(
///             AccessType::ColorWrite.into(),
///             AccessType::ShaderRead.into(),
///         )
///         .build()
///         .expect("Failed to create a barrier.");
///     # }
///
pub trait BarrierBuilder: Send + Sync + Any + Debug + AsRef<Any> + AsMut<Any> {
    /// Define a global memory barrier.
    fn global(
        &mut self,
        src_access: AccessTypeFlags,
        dst_access: AccessTypeFlags,
    ) -> &mut BarrierBuilder;

    /// Define a buffer memory barrier.
    fn buffer(
        &mut self,
        src_access: AccessTypeFlags,
        dst_access: AccessTypeFlags,
        buffer: &Buffer,
        range: Option<Range<DeviceSize>>,
    ) -> &mut BarrierBuilder;

    /// Define an image memory barrier.
    fn image(
        &mut self,
        src_access: AccessTypeFlags,
        dst_access: AccessTypeFlags,
        image: &Image,
        src_layout: ImageLayout,
        dst_layout: ImageLayout,
        range: &ImageSubRange,
    ) -> &mut BarrierBuilder;

    /// Build an `Barrier`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<Barrier>;
}
