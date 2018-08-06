//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Handle types.
//!
//! Handles represent references to objects such as images and shader modules.
//! Handle types are distinguished by the suffix `Ref` and they behave like
//! `Arc`s from the application developer's perspective.
//! They support the following operations:
//!
//!  - `Drop`. Note that dropping a handle does not necessarily destroy the
//!    underlying object. See also the section "Allocation Strategy".
//!  - `Clone`. Only the reference — not the object itself is cloned.
//!
//! There are two kinds of handles:
//!
//!  - *Boxed handles* are `Arc` values each of which represents a reference to
//!    a single heap-allocated object implementing a particular trait.
//!
//!    Trait types which boxed handles are based on provide an interface to
//!    query their concrete types and additional traits implemented by them.
//!    This functionality is provided by the `query_interface` crate.
//!
//!    Note: Boxed handles are previously referred to as just *objects*.
//!
//!  - *Fat handles* store object to the handles themselves. The implementor
//!    must implement `Clone` on the stored objects to emulate the cloning
//!    semantics of `Arc`.
//!
//!    Fat handles encapsulate implementation-dependent objects using
//!    [`SmallBox`]`<_, [usize; 2]>`. Therefore, the contained data must be
//!    sufficiently small to fit `[usize; 2]`.
//!
//!    `HandleImpl` is a trait implemented by all fat handle implementations and
//!    has `AsRef<dyn Any>` in its trait bounds. You can use this to downcast a
//!    handle to a known concrete type.
//!
//! [`SmallBox`]: ../../zangfx_common/struct.SmallBox.html
//!
//! # Allocation Strategy
//!
//! To reduce the run-time cost of tracking the lifetime of objects, ZanGFX
//! requires the application to manually maintain the lifetime of certain
//! object types. Specifically, the following object type is released when
//! and only when the application makes an explicit request to do so:
//! **argument tables**. Argument tables are also released when their
//! containing argument pool is released or resetted.
//!
//! # Examples
//!
//! This example uses the [`zangfx_impl_handle`] macro to define a handle
//! implementation type.
//!
//!     # #[macro_use] extern crate zangfx_base;
//!     # fn main() {
//!     use std::any::Any;
//!     use zangfx_base::{zangfx_impl_handle, CloneHandle, FenceRef};
//!
//!     #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//!     struct MyFence;
//!
//!     zangfx_impl_handle! { MyFence, FenceRef }
//!
//!     let fence = FenceRef::new(MyFence);
//!     assert!(fence.is::<MyFence>());
//!     # }
//!
use std::any::Any;
use std::fmt;

/// Implements the clone behavior of fat handles.
///
/// In most cases, this trait is automatically implemented using the
/// [`zangfx_impl_handle`](zangfx_impl_handle) macro.
///
/// See [the module-level documentation](index.html) for the usage.
pub trait CloneHandle<C>: AsRef<dyn Any> + AsMut<dyn Any> + fmt::Debug + Send + Sync + Any {
    fn clone_handle(&self) -> C;
}

/// Defines a handle type.
macro_rules! define_handle {
    ($(#[$smeta:meta])* $name:ident) => {
        define_handle! { $(#[$smeta])* $name : $crate::handles::CloneHandle<$name> }
    };
    ($(#[$smeta:meta])* $name:ident : $trait:path) => {
        $(#[$smeta])*
        #[derive(Debug)]
        pub struct $name {
            type_id: std::any::TypeId,
            inner: $crate::common::SmallBox<dyn $trait, [usize; 2]>,
        }

        impl $name {
            pub fn new<T>(x: T) -> Self
            where
                T: ::std::marker::Unsize<dyn $trait> + 'static,
            {
                Self {
                    type_id: std::any::TypeId::of::<T>(),
                    inner: unsafe { $crate::common::SmallBox::new(x) },
                }
            }

            pub fn is<T>(&self) -> bool
            where
                T: $trait,
            {
                std::any::TypeId::of::<T>() == self.type_id
            }

            pub fn downcast_ref<T>(&self) -> Option<&T>
            where
                T: $trait,
            {
                if self.is::<T>() {
                    unsafe {
                        Some(&*(&*self.inner as *const _ as *const T))
                    }
                } else {
                    None
                }
            }

            pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
            where
                T: $trait,
            {
                if self.is::<T>() {
                    unsafe {
                        Some(&mut *(&mut *self.inner as *mut _ as *mut T))
                    }
                } else {
                    None
                }
            }
        }

        impl<T> From<T> for $name
        where
            T: ::std::marker::Unsize<dyn $trait> + 'static,
        {
            fn from(x: T) -> Self {
                Self::new(x)
            }
        }

        impl Clone for $name {
            fn clone(&self) -> Self {
                self.inner.clone_handle()
            }
        }

        impl ::std::ops::Deref for $name {
            type Target = dyn $trait;

            fn deref(&self) -> &Self::Target {
                &*self.inner
            }
        }

        impl ::std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut *self.inner
            }
        }
    };
}

/// Generates a boiler-plate code for defining a handle implementation type.
///
/// For a given type, this macro generates the implementation for the following
/// traits: `CloneHandle`, `AsRef<Any>`, and `AsMut<Any>`.
///
/// See [the module-level documentation](index.html) for the usage.
#[macro_export]
macro_rules! zangfx_impl_handle {
    ($type:ty, $handletype:ty) => {
        impl $crate::handles::CloneHandle<$handletype> for $type {
            fn clone_handle(&self) -> $handletype {
                <$handletype>::new(Clone::clone(self))
            }
        }
        impl AsRef<::std::any::Any> for $type {
            fn as_ref(&self) -> &::std::any::Any {
                self
            }
        }
        impl AsMut<::std::any::Any> for $type {
            fn as_mut(&mut self) -> &mut ::std::any::Any {
                self
            }
        }
    };
}
