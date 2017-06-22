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
//! Todos
//! -----
//!
//!  - Use `u64` for device space size/offset

extern crate cgmath;
extern crate enumflags;
#[macro_use]
extern crate enumflags_derive;

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
mod limits;
pub mod pipeline;
pub mod query;
pub mod renderpass;
pub mod sampler;
pub mod shader;
pub mod sync;
pub mod validation;

pub type VertexBindingLocation = usize;
pub type VertexAttributeLocation = usize;

pub type DescriptorSetBindingLocation = usize;
pub type DescriptorBindingLocation = usize;

pub trait Backend: Sized + 'static {
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
    type Framebuffer: Framebuffer;
    type GraphicsPipeline: GraphicsPipeline;
    type Heap: Heap<Self>;
    type Image: Image;
    type ImageView: ImageView;
    type PipelineLayout: PipelineLayout;
    type RenderPass: RenderPass;
    type Sampler: Sampler;
    type SecondaryCommandBuffer: SecondaryCommandBuffer<Self>;
    type Semaphore: Semaphore;
    type StencilState: StencilState;
    type ShaderModule: ShaderModule;
}

// flags are indivdually reexported because "enumflags_derive" generates a struct named "InnerXXX" for each
// enumration type and we do not want it exported
pub use buffer::*;
pub use command::*;
pub use debug::*;
pub use descriptor::*;
pub use error::*;
pub use factory::*;
pub use flags::{AccessFlags, PipelineStageFlags};
pub use formats::*;
pub use framebuffer::*;
pub use geom::*;
pub use heap::*;
pub use image::*;
pub use limits::*;
pub use pipeline::*;
pub use query::*;
pub use renderpass::*;
pub use sampler::*;
pub use shader::*;
pub use sync::*;
pub use validation::*;

pub trait Device<B: Backend>: Debug + Sized {
    fn main_queue(&self) -> &B::CommandQueue;
    fn factory(&self) -> &B::Factory;
    fn capabilities(&self) -> &B::DeviceCapabilities;
}

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
