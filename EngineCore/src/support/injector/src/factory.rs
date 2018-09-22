//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{fmt, fmt::Debug, sync::Arc};

use crate::{Container, Key, SingletonExt};

/// A factory object.
///
/// Only the following forms of the type parameters are accepted:
///
///  - `Factory<K, <K as Key>::Value>` — [`FactoryExt::get_or_build`]
///  - `Factory<(), T>` — [`FactoryExt::get_singleton_or_build`]
///
trait Factory<K, T>: 'static + Send + Sync + Debug {
    fn build(&self, key: &K, container: &mut Container) -> T;
}

type FactoryRef<K, T> = Arc<dyn Factory<K, T>>;

/// Wraps a closure to form a `Factory` object.
struct FactoryImpl<T>(T);

impl<K, T, S> Factory<K, T> for FactoryImpl<S>
where
    S: 'static + Send + Sync + Fn(&K, &mut Container) -> T,
{
    fn build(&self, key: &K, container: &mut Container) -> T {
        self.0(key, container)
    }
}

impl<T> Debug for FactoryImpl<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("FactoryImpl").finish()
    }
}

/// Indicates an error that occured while trying to construct an object using a
/// factory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildError {
    /// The factory object of a specified type or key was not found.
    NoFactory,
}

/// An extension trait for [`crate::Container`] to provide means to register
/// factory objects and use them automatically to instantiate objects on demand.
///
/// See [the crate documentation](index.html) for examples.
pub trait FactoryExt {
    /// Get a mutable reference to an object associated with a specified `key`
    /// and previously registered by [`Container::register`]. Create one using
    /// a factory object registered by [`FactoryExt::register_factory`]`<K>`
    /// if there is not such an object.
    fn get_or_build<K: Key>(&mut self, key: &K) -> Result<&mut K::Value, BuildError>;

    /// Get a mutable reference to an instance of `T` previously registered by
    /// [`Container::register`]. Create one using a factory object registered
    /// by [`FactoryExt::register_singleton_factory`]`<T>` if there is not such
    /// an object.
    fn get_singleton_or_build<T: 'static + Send + Sync + Debug>(
        &mut self,
    ) -> Result<&mut T, BuildError>;

    /// Register a factory that can be used by [`FactoryExt::get_or_build`]`<K>`.
    fn register_factory<K: Key>(
        &mut self,
        factory: impl 'static + Send + Sync + Fn(&K, &mut Container) -> K::Value,
    );

    /// Register a factory that can be used by
    /// [`FactoryExt::get_singleton_or_build`]`<T>`.
    fn register_singleton_factory<T: 'static + Send + Sync + Debug>(
        &mut self,
        factory: impl 'static + Send + Sync + Fn(&mut Container) -> T,
    );
}

impl FactoryExt for Container {
    fn get_or_build<K: Key>(&mut self, key: &K) -> Result<&mut K::Value, BuildError> {
        self.get_or_try_create_with(key, |key, container| {
            let factory: FactoryRef<K, K::Value> =
                Arc::clone(container.get_singleton().ok_or(BuildError::NoFactory)?);
            Ok(factory.build(key, container))
        })
    }

    fn get_singleton_or_build<T: 'static + Send + Sync + Debug>(
        &mut self,
    ) -> Result<&mut T, BuildError> {
        self.get_singleton_or_try_create_with(|container| {
            let factory: FactoryRef<(), T> =
                Arc::clone(container.get_singleton().ok_or(BuildError::NoFactory)?);
            Ok(factory.build(&(), container))
        })
    }

    fn register_factory<K: Key>(
        &mut self,
        factory: impl 'static + Send + Sync + Fn(&K, &mut Container) -> K::Value,
    ) {
        let factory_impl = FactoryImpl(move |key: &_, container: &mut _| factory(key, container));
        let factory: FactoryRef<K, K::Value> = Arc::new(factory_impl);
        self.register_singleton(factory);
    }

    fn register_singleton_factory<T: 'static + Send + Sync + Debug>(
        &mut self,
        factory: impl 'static + Send + Sync + Fn(&mut Container) -> T,
    ) {
        let factory_impl = FactoryImpl(move |_: &_, container: &mut _| factory(container));
        let factory: FactoryRef<(), T> = Arc::new(factory_impl);
        self.register_singleton(factory);
    }
}
