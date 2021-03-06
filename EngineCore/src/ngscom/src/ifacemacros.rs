//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//
// This source code is based on com-rs. The original license text is shown below:
//
//     Copyright (c) 2016 com-rs developers
//
//     Licensed under the Apache License, Version 2.0
//     <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
//     license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
//     option. All files in the project carrying such notice may not be copied,
//     modified, or distributed except according to those terms.
//

/// Macro for generating COM interface definitions.
///
/// # Usage
/// ```
/// use ngscom::{IUnknown, IUnknownTrait, com_iid, com_interface};
///
/// com_iid!(IID_IFOO =
///     [0x12345678, 0x90AB, 0xCDEF, [0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]]);
///
/// com_interface! {
///     interface (IFoo, IFooTrait): (IUnknown, IUnknownTrait) {
///         iid: IID_IFOO,
///         vtable: IFooVtbl,
///
///         fn foo() -> bool;
///     }
/// }
/// # fn main() { }
/// ```
///
/// This example defines an interface called `IFoo`. In this case, the base type is
/// IUnknown, the root COM type. The IID for the interface must also be defined,
/// along with the name of the vtable type, `IFooVtbl`. This isn't publicly exposed,
/// but there is currently no way to generate an ident within a macro so the callee
/// must define one instead.
///
/// The trait `Foo` defines the methods available for the interface, in this case
/// a single method named `foo`. Note that any methods that return no value
/// (e.g. the `void` type in C/C++) should return the unit type `()`.
///
/// ## Inheritance
/// To define interfaces with a deeper hierarchy, add additional parent identifiers
/// to the type definitions. e.g:
///
/// ```
/// # use ngscom::{IUnknown, IUnknownTrait, com_iid, com_interface};
/// # com_iid!(IID_IFOO = [0, 0, 0, [0, 0, 0, 0, 0, 0, 0, 0]]);
/// # com_interface! {
/// #    interface (IFoo, IFooTrait): (IUnknown, IUnknownTrait) {
/// #        iid: IID_IFOO,
/// #        vtable: IFooVtbl,
/// #
/// #        fn foo() -> bool;
/// #    }
/// # }
/// com_iid!(IID_IBAR =
///     [0x12345678, 0x90AB, 0xCDEF, [0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]]);
/// com_interface! {
///     interface (IBar, IBarTrait): (IFoo, IFooTrait), IUnknown {
///         iid: IID_IBAR,
///         vtable: IBarVtbl,
///
///         fn bar(baz: i32) -> ();
///     }
/// }
/// # fn main() { }
/// ```
///
/// This example defines an interface called `IBar` which extends `IFoo` from the
/// previous example. Note that it is necessary to specify the parent types
/// for both the interface and trait declarations.
///
/// The interface hierarchy automates pointer conversion using the `AsComPtr` trait,
/// and the trait hierarchy automatically implements the parent methods for the
/// child interface.
#[macro_export]
macro_rules! com_interface {
    (
        $(#[$iface_attr:meta])*
        interface ($iface:ident, $trait_ident:ident): ($base_iface:ty, $base_trait:path) {
            iid: $iid:ident,
            vtable: $vtable:ident,
            $(
                $(#[$fn_attr:meta])*
                fn $func:ident($($i:ident: $t:ty),*) -> $rt:ty;
            )*
        }
    ) => (
        #[allow(missing_debug_implementations)]
        #[doc(hidden)]
        #[repr(C)]
        pub struct $vtable {
            base: <$base_iface as $crate::ComInterface>::Vtable,
            $($func: extern "C" fn(*mut $iface, $($t),*) -> $rt),*
        }

        $(#[$iface_attr])*
        #[derive(Debug)]
        #[repr(C)]
        pub struct $iface {
            vtable: *const $vtable
        }

        // It's safe to implement Sync/Send because the contents of vtable
        // doesn't actually change
        unsafe impl ::std::marker::Sync for $iface {}
        unsafe impl ::std::marker::Send for $iface {}

        impl $iface {
            $($(#[$fn_attr])*
            pub fn $func(&self $(, $i: $t)*) -> $rt {
                unsafe { ((*self.vtable).$func)(self as *const Self as *mut Self $(,$i)*) }
            })*

            #[doc(hidden)]
            pub fn from_vtable(vtable: *const $vtable) -> Self {
                Self { vtable: vtable }
            }

            #[doc(hidden)]
            pub fn fill_vtable<T, S>() -> $vtable
            where
                T: $trait_ident,
                S: $crate::StaticOffset,
            {
                #[allow(dead_code)]
                struct Thunks;

                impl Thunks {
                    $(extern "C" fn $func<T: $trait_ident, S: $crate::StaticOffset>(this: *mut $iface $(, $i: $t)*) -> $rt {
                        unsafe { T::$func($crate::detail::resolve_parent_object::<S, $iface, T>(this), $($i),*) }
                    })*
                }

                $vtable {
                    base: <$base_iface>::fill_vtable::<T, S>(),
                    $($func: Thunks::$func::<T, S>,)*
                }
            }

            #[doc(hidden)]
            pub fn scan_iid(iid: &$crate::IID) -> bool {
                if $iid == *iid {
                    true
                } else {
                    <$base_iface>::scan_iid(iid)
                }
            }
        }

        pub trait $trait_ident: $base_trait {
            $(fn $func(&self, $($i: $t),*) -> $rt;)*
        }

        impl ::std::ops::Deref for $iface {
            type Target = $base_iface;
            fn deref(&self) -> &$base_iface {
                unsafe { ::std::mem::transmute(self) }
            }
        }

        unsafe impl $crate::AsComPtr<$iface> for $iface {}
        unsafe impl $crate::AsComPtr<$base_iface> for $iface {}

        unsafe impl $crate::ComInterface for $iface {
            #[doc(hidden)]
            type Vtable = $vtable;
            #[doc(hidden)]
            type Trait = dyn $trait_ident;
            #[allow(unused_unsafe)]
            fn iid() -> $crate::IID { unsafe { $iid } }
        }
    );

    (
        $(#[$iface_attr:meta])*
        interface ($iface:ident, $trait_ident:ident): ($base_iface:ty, $base_trait:path), $($extra_base:ty),+ {
            iid: $iid:ident,
            vtable: $vtable:ident,
            $(
                $(#[$fn_attr:meta])*
                fn $func:ident($($i:ident: $t:ty),*) -> $rt:ty;
            )*
        }
    ) => (
        $crate::com_interface! {
            $(#[$iface_attr])*
            interface ($iface, $trait_ident): ($base_iface, $base_trait) {
                iid: $iid,
                vtable: $vtable,
                $($(#[$fn_attr])* fn $func($($i: $t),*) -> $rt;)*
            }
        }

        $(unsafe impl $crate::AsComPtr<$extra_base> for $iface {})*
    )
}

/// Helper macro for defining [`IID`](struct.IID.html) constants.
///
/// # Usage
/// ```
/// use ngscom::com_iid;
/// com_iid!(IID_IFOO = [0, 0, 0, [0, 0, 0, 0, 0, 0, 0, 0]]);
/// # fn main() {}
/// ```
///
/// IIDs are private by default as they are only supposed to be exposed by the
/// `ComPtr::iid` method. If you want to make them public, just add the `pub`
/// keyword before the identifier.
///
/// ```
/// # use ngscom::com_iid;
/// com_iid!(pub IID_IBAR = [0, 0, 0, [0, 0, 0, 0, 0, 0, 0, 0]]);
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! com_iid {
    ($(#[$iid_attr:meta])*
    $name:ident = [$d1:expr, $d2:expr, $d3:expr, $d4:expr $(,)*]) => (
        $(#[$iid_attr])*
        static $name: $crate::IID = $crate::IID {
            data1: $d1,
            data2: $d2,
            data3: $d3,
            data4: $d4,
        };
    );
    ($(#[$iid_attr:meta])*
    pub $name:ident = [$d1:expr, $d2:expr, $d3:expr, $d4:expr $(,)*]) => (
        $(#[$iid_attr])*
        pub static $name: $crate::IID = $crate::IID {
            data1: $d1,
            data2: $d2,
            data3: $d3,
            data4: $d4,
        };
    );
}
