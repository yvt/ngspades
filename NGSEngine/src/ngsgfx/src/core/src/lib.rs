//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Core
//! ===========
//!
//! Defines various types used to interact with a graphics hardware.
//!
//! Safety
//! ------
//!
//! While operations that cause race conditions are said to be unsafe, we don't consider GPU-side race
//! condition unsafe because of the following reasons:
//!
//!  - Due to GPU's highly-pipelined nature, (todo)
//!  - Race conditions might result in an undefined behaviour (e.g., device lost) even in GPUs,
//!    but they are unlikely to cause memory safety violation on the host side.
//!
//! Usage of APIs are validated by the backend. Backend implementors can use the [`Validate`] trait implemented
//! on some descriptor types to validate their usage, but this is usually not enough because some information
//! which are only available to backends are not available to the backend-agnostic validators.
//!
//! TODO: exception safety?
//!
//! [`Validate`]: trait.Validate.html
//!
//! Handles
//! -------
//!
//! Some traits represent *handles*, which means they are used in a manner like
//! `Arc<T>`s instead of actual entities. Handles implement following traits:
//!
//!  - `Clone`: cloning a handle creates a new handle that points the same entity.
//!  - `Send`: combined with `Clone`, this implies the possibility that objects can be accessed from
//!    multiple threads at the same time. In most cases this is not a problem since most objects are
//!    read-only, but in other cases such as where an external synchronization is required by the graphics API,
//!    implementors must use `Mutex` or the `atomic_refcell` crate to ensure the memory safety.
//!    This also implies that handles must be atomically reference counted so the object is not destroyed
//!    as long as there is at least one handle that points it. Users are advised to favor taking a reference to
//!    a handle over cloning it in order to minimize the overhead incurred by atomic operations.
//!  - `Sync`: since cloned handles point the same entity, it should not make any difference to access it from
//!    multiple threads via a single handle.
//!  - `Eq`, `PartialEq`, and `Hash`: These traits are used to establish equivalence relation about
//!    the underlying identities of handles.
//!
//! Following traits represent handles:
//!
//!  - [`Buffer`](trait.Buffer.html)
//!  - (todo)
//!
//! Flags
//! -----
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
//! use ngsgfx_core::{AccessType, AccessTypeFlags};
//!
//! let no_access1: AccessTypeFlags = AccessTypeFlags::empty();
//! let no_access2: AccessTypeFlags = AccessType::empty_bitflag();
//!
//! let oneway_access: AccessTypeFlags = AccessType::TransferRead.into();
//!
//! let twoway_access: AccessTypeFlags =
//!     AccessType::TransferRead |
//!     AccessType::TransferWrite;
//! ```
//!
//! Other Important Concepts
//! ------------------------
//!
//! See module-level or item documentations for other important concepts including:
//!
//!  - Command buffers, queues, device engines, and synchronizations -
//!    see [`::command`](command/index.html)
//!

extern crate cgmath;
extern crate ngsenumflags;
#[macro_use]
extern crate ngsenumflags_derive;
#[macro_use]
extern crate lazy_static;

use std::fmt::Debug;
use std::marker::Sized;

pub mod buffer;
pub mod command;
pub mod debug;
pub mod descriptor;
mod error;
pub mod factory;
mod flags;
pub mod formats;
pub mod framebuffer;
mod geom;
pub mod heap;
pub mod image;
pub mod instance;
mod limits;
pub mod pipeline;
pub mod query;
pub mod renderpass;
pub mod sampler;
pub mod shader;
pub mod sync;
pub mod validation;

/// Represents an index to a subpass in a render pass.
pub type SubpassIndex = usize;

/// Represents a location in a vertex buffer binding table.
pub type VertexBindingLocation = usize;
/// Represents a location of a vertex attribute consumed by a vertex shader.
pub type VertexAttributeLocation = usize;

/// Represents a location in a descriptor set binding table.
pub type DescriptorSetBindingLocation = usize;
/// Represents a location of a descriptor or an array of descriptors in a descriptor set.
pub type DescriptorBindingLocation = usize;
/// Represents an element of an array of descriptors.
pub type DescriptorBindingElementIndex = usize;

/// Represents a device memory size and offset value.
pub type DeviceSize = u64;

pub trait Environment: Sized + Debug + 'static {
    type Backend: Backend;

    type DeviceBuilder: DeviceBuilder<Self>;
    type Instance: Instance<Self>;
    type InstanceBuilder: InstanceBuilder<Self>;
}

/// Aggregates types specific to a backend.
pub trait Backend: Sized + Debug + 'static {
    type Buffer: Buffer;
    type CommandBuffer: CommandBuffer<Self>;
    type CommandQueue: CommandQueue<Self>;
    type ComputePipeline: ComputePipeline;
    type DescriptorPool: DescriptorPool<Self>;
    type DescriptorSet: DescriptorSet<Self>;
    type DescriptorSetLayout: DescriptorSetLayout;
    type Device: Device<Self>;
    type DeviceCapabilities: DeviceCapabilities;
    type Factory: Factory<Self>;
    type Fence: Fence;
    type Event: Event;
    type Framebuffer: Framebuffer;
    type GraphicsPipeline: GraphicsPipeline;
    type UniversalHeap: Heap<Self>;
    type SpecializedHeap: Heap<Self>;
    type Image: Image;
    type ImageView: ImageView;
    type PipelineLayout: PipelineLayout;
    type RenderPass: RenderPass;
    type Sampler: Sampler;
    type SecondaryCommandBuffer: SecondaryCommandBuffer<Self>;
    type StencilState: StencilState;
    type ShaderModule: ShaderModule;

    /// Create a autorelease pool and call the specified function inside it.
    ///
    /// On the macOS platform, the lifetimes of most Cocoa objects are managed by
    /// reference counting. In some cases, the lifetimes of objects are temporarily
    /// extended by inserting references to them into the current autorelease pool
    /// associated with each thread.
    ///
    /// In standard macOS applications, a default autorelease pool is automatically
    /// provided and it is drained at every cycle of the event loop. However,
    /// this is unlikely to be the case in NgsGFX applications. Without an
    /// autorelease pool, autoreleased objects will never get released and you will
    /// leak memory.
    ///
    /// This function provides applications a method to create an
    /// autorelease pool in a cross-platform manner. You must wrap the main event
    /// loop with this function and drain the autorelease pool periodicaly
    /// (by calling `AutoreleasePool::drain`), for example, for every iteration.
    ///
    /// The default implementation just calls the given function with
    /// a mutable reference to [`NullAutoreleasePool`] as the parameter value.
    /// It is expected that the Metal backend is the only backend that provides
    /// a custom implementation of this function.
    ///
    /// [`NullAutoreleasePool`]: struct.NullAutoreleasePool.html
    fn autorelease_pool_scope<T, S>(cb: T) -> S
    where
        T: FnOnce(&mut AutoreleasePool) -> S,
    {
        cb(&mut NullAutoreleasePool)
    }
}

/// An autorelease pool.
///
/// See [`Backend::autorelease_pool_scope`] for more.
///
/// [`Backend::autorelease_pool_scope`]: trait.Backend.html#method.autorelease_pool_scope
pub trait AutoreleasePool {
    fn drain(&mut self);
}

/// The implementation of `AutoreleasePool` for platforms where the management of
/// autorelease pools are unnecessary.
pub struct NullAutoreleasePool;
impl AutoreleasePool for NullAutoreleasePool {
    fn drain(&mut self) {}
}

pub use buffer::*;
pub use command::*;
pub use debug::*;
pub use descriptor::*;
pub use error::*;
pub use factory::*;
pub use flags::*;
pub use formats::*;
pub use framebuffer::*;
pub use geom::*;
pub use heap::*;
pub use image::*;
pub use instance::*;
pub use limits::*;
pub use pipeline::*;
pub use query::*;
pub use renderpass::*;
pub use sampler::*;
pub use shader::*;
pub use sync::*;
pub use validation::*;

/// Represents a physical device.
pub trait Device<B: Backend>: Debug + Sized + Sync + Send {
    fn main_queue(&self) -> &B::CommandQueue;
    fn factory(&self) -> &B::Factory;
    fn capabilities(&self) -> &B::DeviceCapabilities;
}

/// Specifies a predicate (boolean-valued function) on two numeric values
/// used during various kinds of tests (e.g., depth test).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CompareFunction {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}
