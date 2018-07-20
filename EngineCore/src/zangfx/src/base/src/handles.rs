//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! (Light-weight) handle types.
//!
//! Handles represent references to objects such as images and shader modules.
//! Handles are boxed using opaque handle types like [`Image`]. They support
//! the following operations:
//!
//!  - `Drop`. Note that dropping a handle does not necessarily destroy the
//!    underlying object. See also the section "Allocation Strategy".
//!  - `Clone`. Only the reference — not the object itself is cloned.
//!
//! [`Image`]: struct.Image.html
//!
//! Boxing is done using [`SmallBox`]`<_, [usize; 3]>`. Therefore, the contained
//! data must be sufficiently small to fit `[usize; 3]`.
//!
//! [`SmallBox`]: SmallBox
//!
//! # Allocation Strategy
//!
//! To reduce the run-time cost of tracking the lifetime of objects, ZanGFX
//! requires the application to manually maintain the lifetime of certain
//! object types. Specifically, the following object types are released when
//! and only when the application makes an explicit request to do so: **images**,
//! **buffers**, **samplers**, **argument tables**, and **image views**, with
//! the exception of argument tables, which are also released when their
//! originating argument pool is released or resetted.
//!
//! # Examples
//!
//! This example uses the [`zangfx_impl_handle`] macro to define a handle
//! implementation type.
//!
//! [`zangfx_impl_handle`]: macro.zangfx_impl_handle.html
//!
//!     # #[macro_use] extern crate zangfx_base;
//!     # fn main() {
//!     use std::any::Any;
//!     use zangfx_base::{HandleImpl, Image};
//!
//!     #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//!     struct MyImage;
//!
//!     zangfx_impl_handle!(MyImage, Image);
//!
//!     let image = Image::new(MyImage);
//!     assert!(image.is::<MyImage>());
//!     # }
//!
use std::any::Any;
use std::fmt;

/// Base trait for all handle implementation traits.
///
/// See [the module-level documentation](index.html) for the usage.
pub trait HandleImpl<C>
    : AsRef<Any> + AsMut<Any> + fmt::Debug + Send + Sync + Any {
    fn clone_handle(&self) -> C;
}

macro_rules! define_handle {
    ($(#[$smeta:meta])* $name:ident) => {
        $(#[$smeta])*
        #[derive(Debug)]
        pub struct $name {
            inner: ::common::SmallBox<::handles::HandleImpl<$name>, [usize; 3]>,
        }

        impl $name {
            pub fn new<T: ::std::marker::Unsize<::handles::HandleImpl<$name>>>(x: T) -> Self {
                Self {
                    inner: unsafe { ::common::SmallBox::new(x) },
                }
            }

            pub fn is<T: ::handles::HandleImpl<$name>>(&self) -> bool {
                ::std::any::Any::is::<T>((*self.inner).as_ref())
            }

            pub fn downcast_ref<T: ::handles::HandleImpl<$name>>(&self) -> Option<&T> {
                ::std::any::Any::downcast_ref((*self.inner).as_ref())
            }

            pub fn downcast_mut<T: ::handles::HandleImpl<$name>>(&mut self) -> Option<&mut T> {
                ::std::any::Any::downcast_mut((*self.inner).as_mut())
            }
        }

        impl<T: ::std::marker::Unsize<::handles::HandleImpl<$name>>> From<T> for $name {
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
            type Target = ::handles::HandleImpl<$name>;

            fn deref(&self) -> &Self::Target {
                &*self.inner
            }
        }

        impl ::std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut *self.inner
            }
        }
    }
}

/// Generates a boiler-plate code for defining a handle implementation type.
///
/// For a given type, this macro generates the implementation for the following
/// traits: `HandleImpl`, `AsRef<Any>`, and `AsMut<Any>`.
///
/// See [the module-level documentation](index.html) for the usage.
#[macro_export]
macro_rules! zangfx_impl_handle {
    ($type:ty, $handletype:ty) => {
        impl $crate::handles::HandleImpl<$handletype> for $type {
            fn clone_handle(&self) -> $handletype {
                <$handletype>::new(Clone::clone(self))
            }
        }
        impl AsRef<::std::any::Any> for $type {
            fn as_ref(&self) -> &::std::any::Any { self }
        }
        impl AsMut<::std::any::Any> for $type {
            fn as_mut(&mut self) -> &mut ::std::any::Any { self }
        }
    }
}
