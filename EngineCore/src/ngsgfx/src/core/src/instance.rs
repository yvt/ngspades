//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides an abstraction on a connection to API and the underlying system.
use {Environment, Backend, DebugReportTypeFlags, DebugReportHandler};
use std::fmt;
use std::hash::Hash;

pub trait InstanceBuilder<E: Environment>: fmt::Debug + Sync + Send {
    type InitializationError: fmt::Debug;
    type BuildError: fmt::Debug;

    fn new() -> Result<Self, Self::InitializationError>
    where
        Self: Sized;

    /// Register a debug report handler.
    ///
    /// You can provide a custom debug report handler, or you can use one
    /// provided by the crate `ngsgfx_debug`.
    fn enable_debug_report<T: DebugReportHandler + 'static>(
        &mut self,
        _: DebugReportTypeFlags,
        _: T,
    ) {
        // No-op by default
    }

    /// Enable validation layers if available.
    fn enable_validation(&mut self) {
        // No-op by default
    }

    /// Enable the `Marker` and `DebugCommandEncoder` trait if available.
    fn enable_debug_marker(&mut self) {
        // No-op by default
    }

    fn build(&self) -> Result<E::Instance, Self::BuildError>;
}

pub trait Instance<E: Environment>: fmt::Debug + Sync + Send {
    type Adapter: Adapter;

    fn adapters(&self) -> &[Self::Adapter];
    fn default_adapter(&self) -> Option<&Self::Adapter> {
        self.adapters().iter().nth(0)
    }
    fn new_device_builder(&self, adapter: &Self::Adapter) -> E::DeviceBuilder;
}

pub trait Adapter: fmt::Debug + Eq + PartialEq + Hash + Sync + Send + Clone {
    fn name(&self) -> &str;
}

pub trait DeviceBuilder<E: Environment>: fmt::Debug + Sync + Send {
    type BuildError: fmt::Debug;

    fn build(&self) -> Result<<E::Backend as Backend>::Device, Self::BuildError>;
}