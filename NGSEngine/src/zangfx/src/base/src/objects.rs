//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Object type.
use query_interface as qi;

/// Base interface of all ZanGFX objects.
///
/// Use the `interfaces!` macro provided by the `query_interface` crate to
/// implement a ZanGFX object.
///
/// # Examples
///
///     #[macro_use]
///     extern crate zangfx_base;
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
///     zangfx_impl_object! { MyObjectImpl: SomeInterface, Debug, Any }
///
///     impl SomeInterface for MyObjectImpl {}
///
///     # fn main() {
///     let concrete = MyObjectImpl;
///     let boxed: Box<SomeInterface> = Box::new(concrete);
///     assert!(boxed.query_ref::<SomeInterface>().is_some());
///     # }
///
pub trait Object: qi::Object + Sync + Send {}
mopo!(Object);

/// Generates a boiler-plate code for defining a ZanGFX object type.
///
/// For a given type, this macro generates the implementation for `AsObject`.
#[macro_export]
macro_rules! zangfx_impl_object {
    ($type:ty : $($iface:ty),*) => {
        impl $crate::Object for $type {}

        interfaces! { $type: $crate::Object $(, $iface)* }
    }
}

/// Boiler-plate code for defining ZanGFX object trait.
macro_rules! define_object {
    ($t:ty) => {
        mopo! { $t }
        impl ::debug::Label for $t {
            fn label(&mut self, label: &str) -> &mut Self {
                if let Some(x) = self.query_mut::<::debug::SetLabel>() {
                    x.set_label(label);
                }
                self
            }
        }
    }
}
