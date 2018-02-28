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
//!  - `PartialEq`, `Eq` — TODO
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
//!     use zangfx_base::handles::{HandleImpl, Image};
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
use std::{fmt, marker, ops};

use common::SmallBox;
use DeviceSize;

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
            inner: SmallBox<HandleImpl<$name>, [usize; 3]>,
        }

        impl $name {
            pub fn new<T: marker::Unsize<HandleImpl<$name>>>(x: T) -> Self {
                Self {
                    inner: unsafe { SmallBox::new(x) },
                }
            }

            pub fn is<T: HandleImpl<$name>>(&self) -> bool {
                Any::is::<T>((*self.inner).as_ref())
            }

            pub fn downcast_ref<T: HandleImpl<$name>>(&self) -> Option<&T> {
                Any::downcast_ref((*self.inner).as_ref())
            }

            pub fn downcast_mut<T: HandleImpl<$name>>(&mut self) -> Option<&mut T> {
                Any::downcast_mut((*self.inner).as_mut())
            }
        }

        impl<T: marker::Unsize<HandleImpl<$name>>> From<T> for $name {
            fn from(x: T) -> Self {
                Self::new(x)
            }
        }

        impl Clone for $name {
            fn clone(&self) -> Self {
                self.inner.clone_handle()
            }
        }

        impl ops::Deref for $name {
            type Target = HandleImpl<$name>;

            fn deref(&self) -> &Self::Target {
                &*self.inner
            }
        }

        impl ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut *self.inner
            }
        }
    }
}

define_handle! {
    /// Image handle.
    ///
    /// Images are first created using `ImageBuilder`. After an image is created
    /// it is in the **Prototype** state. Before it can be used as an attachment
    /// or a descriptor, it must first be transitioned to the **Allocated**
    /// state by allocating the physical space of the image via a method
    /// provided by `Heap`.
    ///
    /// Once an image is transitioned to the **Allocated** state, it will never
    /// go back to the original state. Destroying the heap where the image is
    /// located causes the image to transition to the **Invalid** state. The
    /// only valid operation to an image in the **Invalid** state is to destroy
    /// the image.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    Image
}

define_handle! {
    /// Buffer handle.
    ///
    /// Buffers are first created using `BufferBuilder`. After a buffer is created
    /// it is in the **Prototype** state. Before it can be used as an attachment
    /// or a descriptor, it must first be transitioned to the **Allocated**
    /// state by allocating the physical space of the buffer via a method
    /// provided by `Heap`.
    ///
    /// Once a buffer is transitioned to the **Allocated** state, it will never
    /// go back to the original state. Destroying the heap where the buffer is
    /// located causes the buffer to transition to the **Invalid** state. The
    /// only valid operation to a buffer in the **Invalid** state is to destroy
    /// the buffer.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    Buffer
}

define_handle! {
    /// Represents a single heap allocation.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    HeapAlloc
}

define_handle! {
    /// Image view object.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    ImageView
}

define_handle! {
    /// Sampler handle.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    Sampler
}

define_handle! {
    /// Fence handle.
    ///
    /// Fences are used for intra-queue synchronization.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    Fence
}

define_handle! {
    /// Barrier handle.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    Barrier
}

define_handle! {
    /// Shader library handle.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    Library
}

define_handle! {
    /// Argument set signature handle.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    ArgTableSig
}

define_handle! {
    /// Argument set handle.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    ArgTable
}

define_handle! {
    /// Root signature handle.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    RootSig
}

define_handle! {
    /// Render pass handle.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    RenderPass
}

define_handle! {
    /// Render target table handle.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    RtTable
}

define_handle! {
    /// Render pipeline handle.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    RenderPipeline
}

define_handle! {
    /// Compute pipeline handle.
    ///
    /// See [the module-level documentation](index.html) for the generic usage
    /// of handles.
    ComputePipeline
}

/// A reference to a resource handle.
///
/// # Examples
///
///     # use zangfx_base::handles::{Image, Buffer, ResourceRef};
///     fn test(image: Image, buffer: Buffer) {
///         let _ref1: ResourceRef = (&image).into();
///         let _ref2: ResourceRef = (&buffer).into();
///     }
///
#[derive(Debug, Clone, Copy)]
pub enum ResourceRef<'a> {
    Image(&'a Image),
    Buffer(&'a Buffer),
}

impl<'a> From<&'a Image> for ResourceRef<'a> {
    fn from(x: &'a Image) -> Self {
        ResourceRef::Image(x)
    }
}

impl<'a> From<&'a Buffer> for ResourceRef<'a> {
    fn from(x: &'a Buffer) -> Self {
        ResourceRef::Buffer(x)
    }
}

/// A reference to a homogeneous slice of handles that can be passed to a shader
/// function as an argument.
///
/// # Examples
///
///     # use zangfx_base::handles::{ImageView, ArgSlice};
///     fn test(image1: ImageView, image2: ImageView) {
///         let _: ArgSlice = [&image1, &image2][..].into();
///     }
///
#[derive(Debug, Clone, Copy)]
pub enum ArgSlice<'a> {
    /// Image views.
    ImageView(&'a [&'a ImageView]),
    /// Buffers and their subranges.
    Buffer(&'a [(ops::Range<DeviceSize>, &'a Buffer)]),
    /// Samplers.
    Sampler(&'a [&'a Sampler]),
}

impl<'a> ArgSlice<'a> {
    pub fn len(&self) -> usize {
        match self {
            &ArgSlice::ImageView(x) => x.len(),
            &ArgSlice::Buffer(x) => x.len(),
            &ArgSlice::Sampler(x) => x.len(),
        }
    }
}

impl<'a> From<&'a [&'a ImageView]> for ArgSlice<'a> {
    fn from(x: &'a [&'a ImageView]) -> Self {
        ArgSlice::ImageView(x)
    }
}

impl<'a> From<&'a [(ops::Range<DeviceSize>, &'a Buffer)]> for ArgSlice<'a> {
    fn from(x: &'a [(ops::Range<DeviceSize>, &'a Buffer)]) -> Self {
        ArgSlice::Buffer(x)
    }
}

impl<'a> From<&'a [&'a Sampler]> for ArgSlice<'a> {
    fn from(x: &'a [&'a Sampler]) -> Self {
        ArgSlice::Sampler(x)
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
