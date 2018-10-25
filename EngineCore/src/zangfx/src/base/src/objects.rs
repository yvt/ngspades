//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Object type.
use query_interface as qi;

use query_interface::mopo;

/// Base interface of all ZanGFX objects.
///
/// # Examples
///
/// The following example shows how to define a new interface based on this
/// type (trait types provided by `zangfx_base`, such as `CmdBuffer`, are
/// basically defined in this way).
///
///     use zangfx_base::{Object, mopo};
///
///     trait SomeInterface: Object {}
///
///     mopo!(SomeInterface);
///
/// See [`zangfx_impl_object`]'s example for how to define a type implementing
/// this newly defined interface.
pub trait Object: qi::Object + Sync + Send {}
mopo!(dyn Object);

/// Generates a boiler-plate code for defining a ZanGFX object type.
///
/// For a given type, this macro generates the implementation for `AsObject`
/// and [`Object`].
///
/// This macro is implemented using the `interfaces!` macro provided by the
/// `query_interface` crate.
///
/// # Examples
///
/// This example defines a type named `MyObjectImpl` implementing
/// `SomeInterface` from [`Object`]'s example code.
///
///     # use zangfx_base::{Object, mopo};
///     # trait SomeInterface: Object {}
///     # mopo!(SomeInterface);
///     use zangfx_base::zangfx_impl_object;
///     use std::fmt::Debug;
///     use std::any::Any;
///
///     #[derive(Debug)]
///     struct MyObjectImpl;
///
///     zangfx_impl_object! { MyObjectImpl:
///         dyn SomeInterface, dyn Debug, dyn Any }
///
///     impl SomeInterface for MyObjectImpl {}
///
///     # fn main() {
///     let concrete = MyObjectImpl;
///     let boxed: Box<SomeInterface> = Box::new(concrete);
///     assert!(boxed.query_ref::<SomeInterface>().is_some());
///     # }
///
#[macro_export]
macro_rules! zangfx_impl_object {
    ($type:ty : $($iface:ty),*) => {
        impl $crate::Object for $type {}

        // For a mysterious reason, `interfaces ! { $type : ... }` does not work
        // since `query_interface` 0.3.4
        $crate::interfaces! { @imp () $type: dyn $crate::Object $(, $iface)* }
    }
}

/// Boiler-plate code for defining ZanGFX object trait.
macro_rules! define_object {
    ($t:ty) => {
        mopo! { $t }
        impl $crate::debug::Label for $t {
            fn label(&mut self, label: &str) -> &mut Self {
                if let Some(x) = self.query_mut::<dyn $crate::debug::SetLabel>() {
                    x.set_label(label);
                }
                self
            }
        }
    }
}
