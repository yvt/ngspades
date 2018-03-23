//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! # ZanGFX – Low-Level Graphics Backend
//!
//! ZanGFX ([zabna] GFX) is a reiteration of NgsGFX, aimed at lower overhead
//! (at cost of safety), ease of use, and faster compilation.
//!
//! [zabna]: http://jbovlaste.lojban.org/dict/zabna
//!
//! # Safety
//!
//! Backend implementations can be categorized into two types depending on their
//! safety level:
//!
//!  - **Safe implementations** do not cause any undefined behaviors as far as
//!    Rust's memory safety is concerned. They can be instantiated via a
//!    non-`unsafe` interface. They usually run considerably slower compared to
//!    the unsafe counterpart due to a horde of run-time checks and validations
//!    required in order to ensure the memory safety.
//!
//!  - **Unsafe implementations** is opposite: they only perform minimum checks,
//!    or maybe do not do any validations at all. They only can be instantiated
//!    via an `unsafe` interface because incorrect usage of them might result in
//!    an undefined behavior.
//!
//! # Terminology
//!
//! ## Mappings with other APIs
//!
//! |          ZanGFX          |         Metal 2        |         Vulkan        |          D3D12          |
//! | ------------------------ | ---------------------- | --------------------- | ----------------------- |
//! | argument                 | argument               | descriptor            | descriptor              |
//! | argument table           | argument buffer        | descriptor set        | descriptor table        |
//! | argument table signature | ?                      | descriptor set layout | ?                       |
//! | argument pool            | (heap and buffer)      | descriptor pool       | descriptor heap         |
//! | root signature           | ?                      | pipeline layout       | root signature          |
//! | command queue            | command queue          | queue                 | ?                       |
//! | command pool             | -                      | command pool          | ?                       |
//! | command buffer           | command buffer         | command buffer        | ?                       |
//! | completion handler       | completed handler      | (fence)               | ?                       |
//! | fence                    | fence                  | event                 | ?                       |
//! | semaphore                | scheduled handler      | semaphore             | ?                       |
//! | library                  | library                | shader module         | ?                       |
//! | buffer                   | buffer                 | buffer                | ?                       |
//! | image                    | texture                | image                 | ?                       |
//! | image view               | (texture view)         | image view            | ?                       |
//! | heap                     | heap                   | device memory         | resource heap           |
//! | render pipeline          | render pipeline state  | graphics pipeline     | graphics pipeline state |
//! | compute pipeline         | compute pipeline state | compute pipeline      | ?                       |
//! | render pass              | (part of RPS)          | render pass           | (part of GPS)           |
//! | render target            | attachment             | attachment            | render target view      |
//! | render target table      | render pass descriptor | framebuffer           | render target views     |
//! | memory type              | storage mode           | memory type           | ?                       |
//! | memory region            | ?                      | memory heap           | ?                       |
//!
//! Note: The mappings shown in this table are very rough. In most cases, a
//! concept from one API does not translate well to another API.
//!
//! ## Abbreviations
//!
//! - **Alloc** - allocation, allocate
//! - **Arg** - argument
//! - **Cmd** - command
//! - **Cmp** - compare
//! - **DS** - depth and/or stencil
//! - **Dst** - destination
//! - **Fn** - function
//! - **Frag** - fragment
//! - **Int** - integer
//! - **Mag** - magnification
//! - **Min** - minification
//! - **Mip** - mipmap, mipmapping
//! - **Norm** - normalize, normalized
//! - **Res** - resource
//! - **Sig** - signature
//! - **Src** - source
//! - **Vec** - vector
//!
//! # Implementation Details
//!
//! ## Flags
//!
//! Parameters that accept multiple flags are defined as [`BitFlags<T>`]
//! (provided by the [`ngsenumflags`] crate) where `T` is an enumerated type
//! (e.g., `AccessType`).
//! For every enumerated type for which such parameters exist, a type alias to
//! `BitFlags<T>` is defined with its name suffixed with `Flags` (e.g.,
//! [`AccessTypeFlags`]).
//!
//! The following example shows how to provide a `BitFlags<T>` value with an
//! arbitrary number of flags (`T` values):
//!
//! ```
//! use zangfx::base::{AccessType, AccessTypeFlags};
//! let no_access1: AccessTypeFlags = AccessTypeFlags::empty();
//! let no_access2: AccessTypeFlags = AccessType::empty_bitflag();
//!
//! let oneway_access: AccessTypeFlags = AccessType::CopyRead.into();
//!
//! let twoway_access_1: AccessTypeFlags =
//!     AccessType::CopyRead |
//!     AccessType::CopyWrite;
//!
//! let twoway_access_2 =
//!     AccessType::CopyRead |
//!     AccessType::CopyWrite;
//! ```
//!
//! Or, by using the `flags!` macro:
//!
//! ```
//! #[macro_use(flags)]
//! extern crate ngsenumflags;
//! # extern crate zangfx;
//! use zangfx::base::AccessType;
//! # fn main() {
//!
//! let no_access = flags![AccessType::{}];
//! let oneway_access = flags![AccessType::{CopyRead}];
//! let twoway_access = flags![AccessType::{CopyRead | CopyWrite}];
//! # }
//! ```
//!
//! [`ngsenumflags`]: ../ngsenumflags/index.html
//! [`BitFlags<T>`]: ../ngsenumflags/struct.BitFlags.html
//! [`AccessTypeFlags`]: ../zangfx_base/type.AccessTypeFlags.html
//!
//! ## Objects
//!
//! The object model of ZanGFX is based around two categories of objects:
//!
//! 1. Normal **objects**. The examples of objects include `Device` and
//!    `CmdQueue`.
//!
//!    Each object provides an interface defined by the trait representing its
//!    object type. The object traits implement `query_ref` and similar methods
//!    (provided by `query_interface`'s [`mopo!`]) via which additional traits
//!    implemented by it can be queried. See the documentation of the crate
//!    [`query_interface`] for details.
//!
//!    Objects are passed around in a boxed form like `Box<Trait>` or
//!    `Arc<Trait>`.
//!
//! 2. Light-weight **handles**. The examples of handles include `Image` and
//!    `Fence`.
//!
//!    Handles do not provide methods by themselves. Instead, they are solely
//!    manipulated via the methods provided by objects.
//!
//!    Handles are capsuled using a type-erasure container type like
//!    `SmallBox<HandleImpl<Image>, S>`. `HandleImpl` is a trait implemented by
//!    all handle implementations and has `AsRef<Any>` in its trait bounds.
//!    You can use this to downcast a handle to a known concrete type.
//!
//!    Some handle types require manual memory management. Others require
//!    a peculiar way to manage their lifetimes. Consult their documentation for
//!    more information.
//!
//! The following table shows all objects and handles defined by ZanGFX as well
//! as the requirements for their manual reference tracking:
//!
//! |         Name        |  Type  |    Is destroyed on    |         Dependents¹         |
//! | ------------------- | ------ | --------------------- | --------------------------- |
//! | `.*Builder`         | object | drop                  |                             |
//! | `Device`            | object | drop                  | GPU and everything          |
//! | `ArgTableSig`       | handle | automatic             |                             |
//! | `RootSig`           | handle | automatic             |                             |
//! | `ArgPool`           | object | drop                  |                             |
//! | `ArgTable`          | handle | Pool `destroy_tables` | GPU, `CmdBuffer`            |
//! |                     |        | Pool `reset`          |                             |
//! |                     |        | Pool `drop`           |                             |
//! | `CmdQueue`          | object | drop                  | GPU, `CmdBuffer`, `CmdPool` |
//! | `CmdPool`           | object | automatic             |                             |
//! | `CmdBuffer`         | object | automatic             |                             |
//! | `Barrier`           | handle | automatic             |                             |
//! | `Fence`             | handle | automatic             |                             |
//! | `Semaphore`         | handle | automatic             |                             |
//! | `RenderPass`        | handle | automatic             |                             |
//! | `RenderTargetTable` | handle | automatic             |                             |
//! | `Heap`              | object | drop                  | `Image`, `Buffer`           |
//! | `HeapAlloc`         | handle | automatic             |                             |
//! | `Image`             | handle | `destroy_image`²      | GPU, `RenderTargetTable`,   |
//! |                     |        |                       | `Barrier`, `ImageView`      |
//! | `Buffer`            | handle | `destroy_buffer`²     | GPU, `ArgTable`, `Barrier`  |
//! | `Sampler`           | handle | `destroy_sampler`     | `ArgTable`                  |
//! | `ImageView`         | handle | `destroy_image_view`  | `ArgTable`                  |
//! | `Library`           | handle | automatic             |                             |
//! | `RenderPipeline`    | handle | automatic             |                             |
//! | `ComputePipeline`   | handle | automatic             |                             |
//!
//! ¹ The **Dependents** column denotes the objects that possibly contain a weak
//! reference to a certain object and require it to operate properly. For example,
//!
//! - Builders are no longer usable (and might cause an *undefined behavior*
//!   if you try to call a method of it) once their parent device was destroyed.
//!
//! - If you submit a command buffer that includes a reference to a buffer. You must
//!   not destroy the buffer nor its heap until the completion of the command
//!   buffer.
//!
//! - You must first wait on queue idle and drop all objects before dropping a
//!   `Device`.
//!
//! Strong/lifetimed references are *not* included in **Dependents**. In other words,
//! those shown in Dependents are the only dependencies you must track manually.
//!
//! ² Images and buffers are invalidated when the heap they were allocated from was
//! destroyed. Invalidated images and buffers are no longer usable, but you still
//! have to explicitly destroy them via `destroy_image` and/or `destroy_buffer`.
//!
//! [`query_interface`]: ../query_interface/index.html
//! [`mopo!`]: ../query_interface/macro.mopo.html
//!
pub extern crate zangfx_base as base;
pub extern crate zangfx_common as common;
pub extern crate zangfx_utils as utils;

/// Includes a backend for each target API.
pub mod backends {
    #[cfg(target_os = "macos")]
    pub extern crate zangfx_metal as metal;

    pub extern crate zangfx_vulkan as vulkan;
}

/// The ZanGFX prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use base::prelude::*;
    #[doc(no_inline)]
    pub use utils::prelude::*;
    #[doc(no_inline)]
    pub use common::{BinaryInteger, BinaryUInteger, FromWithPad, IntoWithPad};
}
