//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Object type.
use std::any::Any;
use std::fmt::Debug;
use query_interface as qi;

// FIXME: Maybe apply `mopo!` on all object types?

/// Base interface of all ZanGFX objects.
///
/// Use the `interfaces!` macro provided by the `query_interface` crate to
/// implement a ZanGFX object.
///
/// # Examples
///
///     #[macro_use]
///     extern crate zangfx_base;
///     #[macro_use]
///     extern crate query_interface;
///
///     use std::fmt::Debug;
///     use std::any::Any;
///
///     trait SomeInterface: zangfx_base::Object {}
///
///     mopo!(SomeInterface);
///
///     #[derive(Debug)]
///     struct MyObjectImpl;
///
///     zangfx_impl_object! { MyObjectImpl }
///     interfaces! { MyObjectImpl: SomeInterface, Debug, Any }
///
///     impl SomeInterface for MyObjectImpl {}
///
///     # fn main() {
///     let concrete = MyObjectImpl;
///     let boxed: Box<SomeInterface> = Box::new(concrete);
///     assert!(boxed.query_ref::<SomeInterface>().is_some());
///     # }
///
pub trait Object: qi::Object + Send + Sync + AsObject + qi::HasInterface<Debug> {}
mopo!(Object);

/// Converts an object to a trait object of `Object`, allowing methods
/// implemented on the trait object (e.g., `query_ref`) to be called.
pub trait AsObject {
    fn as_object(&self) -> &Object;
    fn as_mut_object(&mut self) -> &mut Object;
}

/// Provides `query_ref` and `query_mut` for all `AsObject`s, with different
/// names.
pub trait ObjectQi {
    fn as_ref<U: Any + ?Sized>(&self) -> Option<&U>;
    fn as_mut<U: Any + ?Sized>(&mut self) -> Option<&mut U>;
}

impl<T: AsObject + ?Sized> ObjectQi for T {
    fn as_ref<U: Any + ?Sized>(&self) -> Option<&U> {
        self.as_object().query_ref()
    }

    fn as_mut<U: Any + ?Sized>(&mut self) -> Option<&mut U> {
        self.as_mut_object().query_mut()
    }
}

/// Generates a boiler-plate code for defining a ZanGFX object type.
///
/// For a given type, this macro generates the implementation for `AsObject`.
#[macro_export]
macro_rules! zangfx_impl_object {
    ($type:ty) => {
        impl $crate::Object for $type {}

        impl $crate::objects::AsObject for $type {
            fn as_object(&self) -> &$crate::Object { self }
            fn as_mut_object(&mut self) -> &mut $crate::Object { self }
        }
    }
}
