//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! A library for implementing a software design pattern akin to
//! [dependency injection] or [the service locator pattern].
//!
//! [dependency injection]: https://en.wikipedia.org/wiki/Dependency_injection
//! [the service locator pattern]: https://en.m.wikipedia.org/wiki/Service_locator_pattern
//!
//! # Examples
//!
//! ## Basic usage — Что может [`Container`]?
//!
//! The following example shows what [`Container`] can exactly do:
//!
//!     use injector::{Container, Key};
//!
//!     #[derive(Debug, PartialEq, Eq, Hash, Clone)]
//!     struct MyServiceKey;
//!
//!     #[derive(Debug)]
//!     struct MyService;
//!
//!     impl Key for MyServiceKey {
//!         type Value = MyService;
//!     }
//!
//!     let mut container = Container::new();
//!     container.register(MyServiceKey, MyService);
//!
//!     let _service: &MyService = container.get(&MyServiceKey).unwrap();
//!
//!     // Alternatively, you can supply a factory function:
//!     // (In this example, the factory won't be called because you already
//!     // registered `(MyServiceKey, MyService)` manually.)
//!     let _service: &MyService = container.get_or_create_with(
//!         &MyServiceKey,
//!         |_key, _container| MyService,
//!     );
//!
//! That's it! By itself it may not seem useful, but its strength will be
//! apparent as we move on.
//!
//! *Alternatively, you can just [skip the process and look at the final result](#introducing-singletonext-and-factoryext).*
//!
//! ## Decoupling the client from a concrete implementation
//!
//! This is an application of the so-called [dependency inversion principle].
//!
//! [dependency inversion principle]: https://en.wikipedia.org/wiki/Dependency_inversion_principle
//!
//!     # use injector::{Container, Key};
//!     use std::sync::Arc;
//!
//!     #[derive(Debug, PartialEq, Eq, Hash, Clone)]
//!     struct MyServiceKey;
//!
//!     impl Key for MyServiceKey {
//!         type Value = Arc<dyn MyService>;
//!     }
//!
//!     trait MyService: std::fmt::Debug + Send + Sync {}
//!
//!     #[derive(Debug)]
//!     struct MyServiceImpl;
//!     impl MyService for MyServiceImpl {}
//!
//!     let mut container = Container::new();
//!
//!     // A concrete implementation must be registered somewhere:
//!     container.register(MyServiceKey, Arc::new(MyServiceImpl));
//!
//!     // The client does not need a knowledge of the concrete implementation:
//!     let _service: &Arc<dyn MyService> =
//!         container.get(&MyServiceKey).unwrap();
//!
//! However, this code still has several problems:
//!
//!  - The registration order is dependent on the dependency graph. In other
//!    words, the initialization code still has a somewhat good deal of
//!    knowledge of a specific concrete implementation's internals. For example,
//!    if `MyServiceImpl` depends on another service (let's call this
//!    `YAService`), you have to acknowledge that and register `YAServiceImpl`
//!    before `MyServiceImpl`.
//!  - Instantiated and registered objects might end up being left unused.
//!
//! We'll see in the next example how to alleviate these problems by
//! making the initizaliation process more abstract using this framework.
//!
//! ## The abstract factory pattern
//!
//!     # use injector::{Container, Key};
//!     # use std::sync::Arc;
//!     #[derive(Debug, PartialEq, Eq, Hash, Clone)]
//!     struct MyServiceKey;
//!     impl Key for MyServiceKey {
//!         type Value = Arc<dyn MyService>;
//!     }
//!     trait MyService: std::fmt::Debug + Send + Sync {}
//!
//!     #[derive(Debug, PartialEq, Eq, Hash, Clone)]
//!     struct MyServiceFactoryKey;
//!     impl Key for MyServiceFactoryKey {
//!         type Value = MyServiceFactory;
//!     }
//!     struct MyServiceFactory(
//!         Arc<dyn Fn(&mut Container) -> Arc<dyn MyService> + Send + Sync>
//!     );
//!     impl std::fmt::Debug for MyServiceFactory {
//!         fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//!             write!(f, "[closure]")
//!         }
//!     }
//!
//!     #[derive(Debug)]
//!     struct MyServiceImpl;
//!     impl MyService for MyServiceImpl {}
//!
//!     let mut container = Container::new();
//!
//!     // Register a factory of `MyService`:
//!     container.register(MyServiceFactoryKey, MyServiceFactory(Arc::new(|container| {
//!         // You can define a dependency from `MyServiceImpl` but I think we
//!         // should show that in the next example instead
//!         Arc::new(MyServiceImpl)
//!     })));
//!
//!     // (Optionally register a non-default factory of `MyService` here,
//!     // for example, for testing)
//!
//!     // Instantiate a `MyService` using a registered factory:
//!     let _service: &Arc<dyn MyService> =
//!         container.get_or_create_with(&MyServiceKey, |_key, container| {
//!             // Since the factory itself is stored in the container, we have
//!             // to break the borrow chain before calling the factory
//!             let factory = Arc::clone(&container.get(&MyServiceFactoryKey)
//!                 .expect("factory of MyService was not found").0);
//!             factory(container)
//!         });
//!
//! Whoa, that's a lot of code! But don't you fret for we have two mechanisms
//! to simplify writing this specific coding pattern.
//!
//! ## Introducing [`SingletonExt`] and [`FactoryExt`]
//!
//! In most use cases, you need only a single instance for each type, hence
//! ending up using a unit-like struct as a key to manage such objects as we've
//! seen in the previous examples. To avoid forcing you to write a boiler plate
//! code every time, [`SingletonExt`] provides extension methods that allows you
//! to directly use the type of the stored object as a key.
//! (See the documentation of [`SingletonExt`] for an example showing how to
//! use `SingletonExt` alone.)
//!
//! [`FactoryExt`] implements the aforementioned factory pattern. It also
//! incorporates the singleton pattern of `SingletonExt`. Using this, we can
//! rewrite the previous example to something like this:
//!
//!     use injector::{Container, FactoryExt};
//!     # use std::sync::Arc;
//!
//!     trait MyService: std::fmt::Debug + Send + Sync {}
//!     type MyServiceRef = Arc<dyn MyService>;
//!
//!     #[derive(Debug)]
//!     struct MyServiceImpl;
//!     impl MyService for MyServiceImpl {}
//!
//!     let mut container = Container::new();
//!
//!     // Register a factory of `MyService`:
//!     container.register_singleton_factory(
//!         |container: &mut Container| -> MyServiceRef {
//!             Arc::new(MyServiceImpl)
//!         });
//!
//!     // Instantiate a `MyService` using a registered factory:
//!     let _service: &mut MyServiceRef = container
//!         .get_singleton_or_build::<MyServiceRef>()
//!         // At this point we have either `Ok(&mut x: MyServiceRef)` or
//!         // `Err(BuildError::NoFactory)`
//!         .expect("We don't know how to make MyService.");
//!
//! Now it's much better, isn't it? Let's add another service to show how we
//! handle dependencies:
//!
//!     # use injector::{Container, FactoryExt};
//!     # use std::sync::Arc;
//!     # trait MyService: std::fmt::Debug + Send + Sync {}
//!     # type MyServiceRef = Arc<dyn MyService>;
//!     # #[derive(Debug, PartialEq, Eq)] struct MyServiceImpl;
//!     # impl MyService for MyServiceImpl {}
//!     # let mut container = Container::new();
//!     # container.register_singleton_factory(
//!     #     |container: &mut Container| -> MyServiceRef {
//!     #         Arc::new(MyServiceImpl)
//!     #     });
//!     trait YAService: std::fmt::Debug + Send + Sync {}
//!     type YAServiceRef = Arc<dyn YAService>;
//!
//!     #[derive(Debug)]
//!     struct YAServiceImpl(MyServiceRef);
//!     impl YAService for YAServiceImpl {}
//!
//!     container.register_singleton_factory(
//!         |container: &mut Container| -> YAServiceRef {
//!             // Our implementation of `YAServiceImpl` requires a reference to
//!             // `MyService`, so request one via the supplied mutable reference
//!             // to the container.
//!             let my_service = Arc::clone(container
//!                 .get_singleton_or_build::<MyServiceRef>()
//!                 .expect("We don't know how to make MyService."));
//!
//!             // Now that we have all required dependencies, we can
//!             // instantiate `YAServiceImpl`.
//!             Arc::new(YAServiceImpl(my_service))
//!         });
//!
//!     let _service: &mut YAServiceRef = container
//!         .get_singleton_or_build::<YAServiceRef>()
//!         .expect("We don't know how to make YAService.");
//!
//! ## Error handling
//!
//! One possible way to handle creation errors is to replace the return types of
//! factory methods with `Result<T, E>`.
//!
//!     use injector::{Container, FactoryExt};
//!     # use std::sync::Arc;
//!
//!     #[derive(Debug, Clone)]
//!     struct Error;
//!
//!     trait MyService: std::fmt::Debug + Send + Sync {}
//!     type MyServiceRef = Arc<dyn MyService>;
//!
//!     #[derive(Debug)]
//!     struct MyServiceImpl;
//!     impl MyService for MyServiceImpl {}
//!
//!     trait YAService: std::fmt::Debug + Send + Sync {}
//!     type YAServiceRef = Arc<dyn YAService>;
//!
//!     #[derive(Debug)]
//!     struct YAServiceImpl(MyServiceRef);
//!     impl YAService for YAServiceImpl {}
//!
//!     let mut container = Container::new();
//!
//!     // Register factories
//!     container.register_singleton_factory(
//!         |container: &mut Container| -> Result<MyServiceRef, Error> {
//!             Err(Error)
//!         });
//!
//!     container.register_singleton_factory(
//!         |container: &mut Container| -> Result<YAServiceRef, Error> {
//!             // Propagate an error returned from `MyService`'s factory if any
//!             let my_service = container
//!                 .get_singleton_or_build::<Result<MyServiceRef, Error>>()
//!                 .expect("We don't know how to make MyService.")
//!                 .clone()  // Get `Result<MyServiceRef, Error>`
//!                 ?;        // Bail out if we get `Err(_)`
//!
//!             Ok(Arc::new(YAServiceImpl(my_service)))
//!         });
//!
//!     // Instantiate a `YAService` using a registered factory:
//!     container
//!         .get_singleton_or_build::<Result<YAServiceRef, Error>>()
//!         .expect("We don't know how to make YAService.")
//!         .clone()  // Get `Result<YAServiceRef, Error>`
//!         .expect_err("The error did not propagate for some reasons");
//!
#![feature(never_type)]
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    hash::Hash,
    mem::replace,
};

mod factory;
mod singleton;

pub use self::factory::*;
pub use self::singleton::*;

/// The `injector` prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use super::{FactoryExt, SingletonExt};
}

/// A DI-like container.
///
/// See [the crate documentation](index.html) for details.
#[derive(Default, Debug)]
pub struct Container {
    /// Each element is a `ValueBag<K, K::Value>` where `K: Key`.
    key_types: HashMap<TypeId, Box<dyn ValueBagTrait>>,
}

/// Identifies an object in a [`Container`].
pub trait Key: Any + Send + Sync + Hash + Eq + Clone + fmt::Debug {
    /// The type of the object to be stored in a [`Container`], associated with
    /// this (or `Eq`uivalent) `Key`.
    type Value: Send + Sync + fmt::Debug;
}

impl Container {
    /// Construct an empty `Container`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a reference to an object associated with a specified `key` and
    /// previously registered by [`Container::register`].
    ///
    /// Returns `None` if there is not such an object.
    pub fn get<K: Key>(&self, key: &K) -> Option<&K::Value> {
        let key_type_map: &ValueBag<K, K::Value> = self
            .key_types
            .get(&TypeId::of::<K>())?
            .as_any()
            .downcast_ref()
            .unwrap();
        key_type_map.get(key)
    }

    /// Get a mutable reference to an object associated with a specified `key`
    /// and previously registered by [`Container::register`].
    ///
    /// Returns `None` if there is not such an object.
    pub fn get_mut<K: Key>(&mut self, key: &K) -> Option<&mut K::Value> {
        let key_type_map: &mut ValueBag<K, K::Value> = self
            .key_types
            .get_mut(&TypeId::of::<K>())?
            .as_any_mut()
            .downcast_mut()
            .unwrap();
        key_type_map.get_mut(key)
    }

    /// Get a mutable reference to an object associated with a specified `key`
    /// and previously registered by [`Container::register`]. Create one using
    /// `factory` if there is not such an object.
    pub fn get_or_create_with<K: Key>(
        &mut self,
        key: &K,
        factory: impl FnOnce(&K, &mut Self) -> K::Value,
    ) -> &mut K::Value {
        self.get_or_try_create_with(key, |key, this| Ok(factory(key, this)) as Result<_, !>)
            .unwrap()
    }

    /// Get a mutable reference to an object associated with a specified `key`
    /// and previously registered by [`Container::register`]. Create one using
    /// `factory` if there is not such an object.
    ///
    /// `factory` may fail with an error type `E`.
    pub fn get_or_try_create_with<K: Key, E>(
        &mut self,
        key: &K,
        factory: impl FnOnce(&K, &mut Self) -> Result<K::Value, E>,
    ) -> Result<&mut K::Value, E> {
        // Work-around borrow check issue
        // if let Some(x) = self.get_mut(key) {
        //     return Ok(x);
        // }
        if let Some(_) = self.get_mut(key) {
            return Ok(self.get_mut(key).unwrap());
        }

        let value = factory(key, self)?;

        let key_type_map_entry = self.key_types.entry(TypeId::of::<K>());

        let key_type_map: &mut ValueBag<K, K::Value> = key_type_map_entry
            .or_insert_with(|| {
                let key_type_map: ValueBag<K, K::Value> = ValueBag::new();
                Box::new(key_type_map)
            }).as_any_mut()
            .downcast_mut()
            .unwrap();

        Ok(key_type_map.insert(key.clone(), value).0)
    }

    /// Register an object associated with a specified `key`.
    ///
    /// Returns the previously registered object with an identical key, if any.
    pub fn register<K: Key>(&mut self, key: K, value: K::Value) -> Option<K::Value> {
        let key_type_map_entry = self.key_types.entry(TypeId::of::<K>());

        let key_type_map: &mut ValueBag<K, K::Value> = key_type_map_entry
            .or_insert_with(|| {
                let key_type_map: ValueBag<K, K::Value> = ValueBag::new();
                Box::new(key_type_map)
            }).as_any_mut()
            .downcast_mut()
            .unwrap();

        key_type_map.insert(key, value).1
    }
}

enum ValueBag<K: Eq + Hash, V> {
    Empty,
    Singleton(K, V),
    Generic(HashMap<K, V>),
}

// Type-erasing trait of `ValueBag`
trait ValueBagTrait: fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<K: Eq + Hash, V> ValueBagTrait for ValueBag<K, V>
where
    K: 'static + fmt::Debug + Send + Sync,
    V: 'static + fmt::Debug + Send + Sync,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Make `ValueBag` look as if it were a mere `HashMap`
impl<K: Eq + Hash, V> fmt::Debug for ValueBag<K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ValueBag::*;

        match self {
            Empty => f.debug_map().finish(),
            Singleton(k, v) => f.debug_map().entry(k, v).finish(),
            Generic(map) => f.debug_map().entries(map.iter()).finish(),
        }
    }
}

impl<K: Eq + Hash, V> ValueBag<K, V> {
    fn new() -> Self {
        ValueBag::Empty
    }

    fn insert(&mut self, key: K, value: V) -> (&mut V, Option<V>) {
        use self::ValueBag::*;

        if let Empty = self {
            *self = Singleton(key, value);
            return match self {
                Singleton(_, value) => (value, None),
                _ => unreachable!(),
            };
        } else if let Singleton(_, _) = self {
            if let Singleton(old_key, old_value) = replace(self, Empty) {
                *self = Generic(Some((old_key, old_value)).into_iter().collect());
            } else {
                unreachable!();
            }
        }

        match self {
            Generic(map) => {
                use std::collections::hash_map::Entry;

                match map.entry(key) {
                    Entry::Vacant(e) => (e.insert(value), None),
                    Entry::Occupied(mut e) => {
                        let old_value = replace(e.get_mut(), value);
                        (e.into_mut(), Some(old_value))
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    fn get(&self, key: &K) -> Option<&V> {
        use self::ValueBag::*;

        match self {
            Empty => None,
            Singleton(k, v) => if k == key {
                Some(v)
            } else {
                None
            },
            Generic(map) => map.get(key),
        }
    }

    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        use self::ValueBag::*;

        match self {
            Empty => None,
            Singleton(k, v) => if k == key {
                Some(v)
            } else {
                None
            },
            Generic(map) => map.get_mut(key),
        }
    }
}
