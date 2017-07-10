//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides an abstraction on a connection to API and the underlying system.
use {Environment, Backend};
use std::fmt;
use std::hash::Hash;

pub trait InstanceBuilder<E: Environment>: fmt::Debug + Sync + Send {
    type InitializationError: fmt::Debug;
    type BuildError: fmt::Debug;

    fn new() -> Result<Self, Self::InitializationError>
    where
        Self: Sized;
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
