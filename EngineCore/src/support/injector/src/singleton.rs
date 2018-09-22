//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{fmt::Debug, marker::PhantomData};

use crate::{Container, Key};

struct SingletonKey<T>(PhantomData<T>);

unsafe impl<T> Send for SingletonKey<T> {}
unsafe impl<T> Sync for SingletonKey<T> {}

impl<T> std::fmt::Debug for SingletonKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("SingletonKey").finish()
    }
}

impl<T> PartialEq for SingletonKey<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<T> Eq for SingletonKey<T> {}

impl<T> std::hash::Hash for SingletonKey<T> {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

impl<T> Default for SingletonKey<T> {
    fn default() -> Self {
        SingletonKey(PhantomData)
    }
}

impl<T> Clone for SingletonKey<T> {
    fn clone(&self) -> Self {
        Default::default()
    }
}

impl<T: 'static + Send + Sync + Debug> Key for SingletonKey<T> {
    type Value = T;
}

/// Get a `Key` object for a specified value type.
///
/// [`SingletonExt`] uses this function to supply a `Key`.
pub fn singleton_key<T: 'static + Send + Sync + Debug>() -> impl Key<Value = T> {
    SingletonKey::<T>::default()
}

/// An extension trait for [`crate::Container`] for accessing singleton
/// objects (i.e. only one instance of a type can exist in a single `Container`).
///
/// These methods are merely wrappers that automatically supplies
/// [`singleton_key`]`<T>` as the key.
///
/// # Examples
///
///     use injector::{Container, SingletonExt};
///
///     #[derive(Debug)]
///     struct MyService;
///
///     let mut container = Container::new();
///
///     // Register an instance of `MyService`:
///     container.register_singleton::<MyService>(MyService);
///
///     // Get a reference to the instance:
///     let _service = container.get_singleton::<MyService>()
///         .expect("MyService is not in the container");
///
pub trait SingletonExt {
    /// Get a reference to an instance of`T` previously registered by
    /// [`SingletonExt::register_singleton`].
    ///
    /// Returns `None` if there is not such an object.
    fn get_singleton<T: 'static + Send + Sync + Debug>(&self) -> Option<&T>;

    /// Get a mutable reference to an instance of `T` previously registered by
    /// [`SingletonExt::register_singleton`].
    ///
    /// Returns `None` if there is not such an object.
    fn get_singleton_mut<T: 'static + Send + Sync + Debug>(&mut self) -> Option<&mut T>;

    /// Get a mutable reference to an instance of `T` previously registered by
    /// [`SingletonExt::register_singleton`]. Create one using `factory` if
    /// there is not such an object.
    fn get_singleton_or_create_with<T: 'static + Send + Sync + Debug>(
        &mut self,
        factory: impl FnOnce(&mut Self) -> T,
    ) -> &mut T;

    /// Get a mutable reference to an instance of `T` previously registered by
    /// [`SingletonExt::register_singleton`]. Create one using `factory` if
    /// there is not such an object.
    ///
    /// `factory` may fail with an error type `E`.
    fn get_singleton_or_try_create_with<T: 'static + Send + Sync + Debug, E>(
        &mut self,
        factory: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<&mut T, E>;

    /// Register an instance of `T`.
    ///
    /// Returns the previously registered object with an identical type, if any.
    fn register_singleton<T: 'static + Send + Sync + Debug>(&mut self, value: T) -> Option<T>;
}

impl SingletonExt for Container {
    fn get_singleton<T: 'static + Send + Sync + Debug>(&self) -> Option<&T> {
        self.get(&singleton_key::<T>())
    }

    fn get_singleton_mut<T: 'static + Send + Sync + Debug>(&mut self) -> Option<&mut T> {
        self.get_mut(&singleton_key::<T>())
    }

    fn get_singleton_or_create_with<T: 'static + Send + Sync + Debug>(
        &mut self,
        factory: impl FnOnce(&mut Self) -> T,
    ) -> &mut T {
        self.get_or_create_with(&singleton_key::<T>(), |_, this| factory(this))
    }

    fn get_singleton_or_try_create_with<T: 'static + Send + Sync + Debug, E>(
        &mut self,
        factory: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<&mut T, E> {
        self.get_or_try_create_with(&singleton_key::<T>(), |_, this| factory(this))
    }

    fn register_singleton<T: 'static + Send + Sync + Debug>(&mut self, value: T) -> Option<T> {
        self.register(singleton_key::<T>(), value)
    }
}
