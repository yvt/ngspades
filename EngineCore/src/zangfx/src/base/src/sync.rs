//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for synchronization objects.
use crate::{Object, Result};

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

/// The builder object for semaphores.
pub type SemaphoreBuilder = Box<dyn SemaphoreBuilderTrait>;

/// Trait for building semaphores.
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
pub trait SemaphoreBuilderTrait: Object {
    /// Build an `Semaphore`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<Semaphore>;
}

/// An implementation of `SemaphoreBuilderTrait` that always panics when `build` is
/// called.
#[derive(Debug)]
pub struct NotSupportedSemaphoreBuilder;

zangfx_impl_object! {
    NotSupportedSemaphoreBuilder: SemaphoreBuilderTrait,
    ::std::fmt::Debug
}

impl SemaphoreBuilderTrait for NotSupportedSemaphoreBuilder {
    fn build(&mut self) -> Result<Semaphore> {
        panic!("not supported by this backend")
    }
}
