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
//! | completion handler       | completed handler      | (fence)               | ?                       |
//! | fence                    | fence                  | event                 | ?                       |
//! | semaphore                | scheduled handler      | semaphore             | ?                       |
//! | library                  | library                | shader module         | ?                       |
//! | buffer                   | buffer                 | buffer                | ?                       |
//! | image                    | texture                | image + image view    | ?                       |
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
//! - **Ref** - reference
//! - **Res** - resource
//! - **Sig** - signature
//! - **Src** - source
//! - **Vec** - vector
//!
//! # Implementation Details
//!
//! ## Flags
//!
//! Types representing a subset of predetermined values are defined using
//! the [`bitflags`] crate. Such types usually have `Flags` as suffix in their
//! names (e.g., [`AccessTypeFlags`]).
//!
//! The following example shows how to use such types:
//!
//! ```
//! use zangfx::base::AccessTypeFlags;
//! let no_access: AccessTypeFlags = AccessTypeFlags::empty();
//!
//! let oneway_access: AccessTypeFlags = AccessTypeFlags::CopyRead;
//!
//! let twoway_access: AccessTypeFlags =
//!     AccessTypeFlags::CopyRead |
//!     AccessTypeFlags::CopyWrite;
//! ```
//!
//! Or, by using the [`flags!`](flags_macro::flags) macro:
//!
//! ```
//! #[macro_use(flags)]
//! extern crate flags_macro;
//! # extern crate zangfx;
//! use zangfx::base::AccessTypeFlags;
//! # fn main() {
//!
//! let no_access = flags![AccessTypeFlags::{}];
//! let oneway_access = flags![AccessTypeFlags::{CopyRead}];
//! let twoway_access = flags![AccessTypeFlags::{CopyRead | CopyWrite}];
//! # }
//! ```
//!
//! [`bitflags`]: ../bitflags/index.html
//! [`AccessTypeFlags`]: ../zangfx_base/type.AccessTypeFlags.html
//!
//! ## Objects
//!
//! Objects in ZanGFX are reified as memory-based objects. Each object type
//! optionally provides a corresponding trait type defining operations allowed
//! on the object type.
//!
//! The object model of ZanGFX is mainly based around two categories of objects:
//!
//! 1. **Handle-based objects**. The examples of objects include `Device`,
//!    `Image`, and `CmdQueue`. They are passed around in a boxed form like
//!    `DeviceRef` (which is a type alias for `Arc<dyn Device>`).
//!
//!    Handles can be further divided into two types depending on
//!    how exactly they are boxed. See the module-level documentation of
//!    [`zangfx_base::handles`] for more details.
//!
//! 2. **Unsynchronized objects**. The examples of unsynchronized objects
//!    include `CmdBuffer`. As with handle-based objects, they are passed around
//!    in a boxed form like `CmdBufferRef` (which is a type alias for
//!    `Box<dyn CmdBuffer>`). The difference is that unsynchronized objects do
//!    not use `Arc`-like cloning behaviors or internal synchronization
//!    mechanism but rather rely on Rust's language features to protect them
//!    from simultaneous updates from multiple threads.
//!
//! For the objects *except for* those having fat handle types, their traits
//! implement `query_ref` and similar methods (provided by `query_interface`'s
//! [`mopo!`]) via which additional traits implemented by it can be queried.
//! See the documentation of the crate [`query_interface`] for details.
//!
//! The following table shows all object types defined by ZanGFX:
//!
//! |        Trait        |      Type      | Invalidated when¹  |
//! | ------------------- | -------------- | ------------------ |
//! | `.*Builder`         | unsynchronized |                    |
//! | `Device`            | boxed handle   |                    |
//! | `ArgTableSig`       | fat handle     |                    |
//! | `RootSig`           | fat handle     |                    |
//! | `ArgPool`           | boxed handle   |                    |
//! | `ArgTable`          | fat handle     | `ArgPool` methods² |
//! | `CmdQueue`          | boxed handle   |                    |
//! | `CmdBuffer`         | unsynchronized |                    |
//! | `Fence`             | fat handle     |                    |
//! | `Semaphore`         | fat handle     |                    |
//! | `RenderPass`        | fat handle     |                    |
//! | `RenderTargetTable` | fat handle     |                    |
//! | `Heap`              | boxed handle   |                    |
//! | `Image`             | fat handle     |                    |
//! | `Buffer`            | fat handle     |                    |
//! | `Sampler`           | fat handle     |                    |
//! | `Library`           | fat handle     |                    |
//! | `RenderPipeline`    | fat handle     |                    |
//! | `ComputePipeline`   | fat handle     |                    |
//!
//! ¹ The **Invalidated when** column denotes the actions that render objects
//! of that type invalid when executed.
//!
//! ² `ArgTable` points a region allocated inside an `ArgPool`. `ArgTable`s are
//! invalidated when the region was explicitly deallocated by calling methods on
//! `ArgPool`, or when an `ArgPool` containing them is dropped.
//!
//! ## Inter-queue operation
//!
//! Each image, buffer, dedicated heap (with `use_heap` enabled), fence, and
//! argument pool object is associated with a single queue.
//! The automatic resource state tracking works on the per-queue basis — it does
//! not have knowledge outside a single queue.
//!
//! The queue to which an object belongs is specified as a part of the object
//! creation parameter. The default value is defined in a backend-specific
//! fashion.
//!
//! The application must create a *proxy object* to use it from a different
//! queue. Furthermore, the application must perfom appropriate synchronization.
//! Specifically, in order to use objects in a different queue from one where
//! they were previously used, the application must do the following:
//!
//! - Use semaphores or command buffer completion callbacks to ensure the proper
//!   ordering of command buffer execution.
//! - Perform a *queue family ownership transfer operation*. This includes:
//!     - Executing a *queue family ownership release operation* on the
//!       source queue.
//!     - Executing a *queue family ownership acquire operation* on the
//!       destination queue.
//!
//! The following objects and operations require matching associated queues:
//!
//!  - Heaps passed to `use_heap`.
//!  - Resources (buffers/images) passed to any methods of `*CmdEncoder`
//!    and `CmdBuffer` that accept them, which include but are not limited to:
//!    `use_resource`, `draw_indirect`, and `host_barrier`.
//!  - Render target images (specified via `RenderTargetTableBuilder`) passed to
//!    `encoder_render`.
//!  - Fences passed to `update_fence` or `wait_fence`. (Note: Proxies cannot be
//!    created for fences.)
//!  - Resources passed to `DedicatedHeapBuilder::bind` if `use_heap` is enabled
//!    on the created heap.
//!
//! Some operations (e.g., `use_heap`) do not work on proxy objects.
//!
//! Every object can have up to one proxy or original object created for each
//! queue.
//! Creating more is not allowed and might lead to an undefined behavior on
//! unsafe backend implementations.
//!
//! [`zangfx_base::handles`]: ../zangfx_base/handles/index.html
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
    pub use common::{BinaryInteger, BinaryUInteger, FromWithPad, IntoWithPad};
    #[doc(no_inline)]
    pub use utils::prelude::*;
}
