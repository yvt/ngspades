//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for synchronization objects.
use crate::resources::{Buffer, Image, ImageLayout, ImageSubRange};
use crate::{AccessTypeFlags, DeviceSize};
use crate::{Object, Result};
use std::ops::Range;

define_handle! {
    /// Fence handle.
    ///
    /// Fences are used for intra-queue synchronization.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    Fence
}

define_handle! {
    /// Semaphore handle.
    ///
    /// Fences are used for inter-queue/API synchronization. Not supported by
    /// every backend.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    Semaphore
}

define_handle! {
    /// Barrier handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    Barrier
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
///             AccessType::FragmentRead.into(),
///         )
///         .build()
///         .expect("Failed to create a barrier.");
///     # }
///
pub trait BarrierBuilder: Object {
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

/// Trait for building semaphores.
///
/// # Valid Usage
///
///  - No instance of `SemaphoreBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # fn test(device: &Device) {
///     let semaphore = device.build_semaphore()
///         .build()
///         .expect("Failed to create a semaphore.");
///     # }
///
pub trait SemaphoreBuilder: Object {
    /// Build an `Semaphore`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<Semaphore>;
}

/// An implementation of `SemaphoreBuilder` that always panics when `build` is
/// called.
#[derive(Debug)]
pub struct NotSupportedSemaphoreBuilder;

zangfx_impl_object! { NotSupportedSemaphoreBuilder:
SemaphoreBuilder, ::std::fmt::Debug }

impl SemaphoreBuilder for NotSupportedSemaphoreBuilder {
    fn build(&mut self) -> Result<Semaphore> {
        panic!("not supported by this backend")
    }
}
