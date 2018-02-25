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
//! | command buffer           | command buffer         | command buffer        | ?                       |
//! | event                    | (CB callbacks)         | fence                 | ?                       |
//! | fence                    | fence                  | event                 | ?                       |
//! | library                  | library                | shader module         | ?                       |
//! | heap                     | heap                   | device memory         | resource heap           |
//! | render pipeline          | render pipeline state  | graphics pipeline     | graphics pipeline state |
//! | compute pipeline         | compute pipeline state | compute pipeline      | ?                       |
//! | render pass              | (part of RPS)          | render pass           | (part of GPS)           |
//! | render target            | attachment             | attachment            | render target view      |
//! | render target table      | render pass descriptor | framebuffer           | render target views     |
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
//! - **Rt** - render target
//! - **Sig** - signature
//! - **Src** - source
//! - **Vec** - vector
//!
//! # Flags
//!
//! Parameters that accept multiple flags are defined as `BitFlags<T>` (provided by
//! the `ngsenumflags` crate) where `T` is an enumerated type (e.g., `AccessType`).
//! For every enumerated type for which such parameters exist, a type alias to
//! `BitFlags<T>` is defined with its name suffixed with `Flags` (e.g., `AccessTypeFlags`).
//!
//! There are some exceptions including `ImageFlags`, which is a type alias of
//! `BitFlags<ImageFlagsBit>`.
//!
//! The following example shows how to provide a `BitFlags<T>` value with an arbitrary
//! number of `T` values:
//!
//! ```
//! use zangfx_base::{AccessType, AccessTypeFlags};
//!
//! let no_access1: AccessTypeFlags = AccessTypeFlags::empty();
//! let no_access2: AccessTypeFlags = AccessType::empty_bitflag();
//!
//! let oneway_access: AccessTypeFlags = AccessType::TransferRead.into();
//!
//! let twoway_access: AccessTypeFlags =
//!     AccessType::CopyRead |
//!     AccessType::CopyWrite;
//! ```
pub extern crate zangfx_base as base;
pub extern crate zangfx_common as common;

/// The ZanGFX prelude.
#[doc(no_inline)]
pub mod prelude {
    pub use base::prelude::*;
}
