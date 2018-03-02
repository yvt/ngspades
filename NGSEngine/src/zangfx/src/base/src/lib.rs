//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! This crate is a part of [ZanGFX](../zangfx/index.html) and provides the base
//! interface for backend implementations.
#![feature(unsize)]
#![feature(macro_reexport)]

#[macro_use]
extern crate lazy_static;
extern crate ngsenumflags;
#[macro_use]
extern crate ngsenumflags_derive;
#[macro_use]
#[macro_reexport(interfaces, vtable_for, mopo)] // FIXME: deprecated in favor of Macro 2.0
extern crate query_interface;
extern crate zangfx_common as common;

// `handles` defines a macro
pub mod handles;
// It's here for no reasons
pub mod objects;

pub mod arg;
pub mod command;
pub mod device;
mod flags;
pub use self::flags::*;
pub mod formats;
pub mod heap;
pub mod limits;
pub mod pass;
pub mod pipeline;
pub mod resources;
pub mod sampler;
pub mod shader;
pub mod sync;

/// Represents a device memory size and offset value.
pub type DeviceSize = u64;

/// Represents a queue family index of a specific device.
pub type QueueFamily = u32;

/// Represents a memory type index of a specific device.
pub type MemoryType = u32;

/// Represents a memory region index of a specific device.
pub type MemoryRegionIndex = u32;

/// Represents an index to a subpass in a render pass.
pub type SubpassIndex = usize;

/// Represents a location in a vertex buffer binding table.
pub type VertexBindingLocation = usize;
/// Represents a location of a vertex attribute consumed by a vertex shader.
pub type VertexAttrLocation = usize;

/// Represents a location of an argument table in an argument binding table.
pub type ArgTableIndex = usize;
/// Represents an argument location in an argument table.
pub type ArgIndex = usize;
/// Represents an element of an array of descriptors.
pub type ArgArrayIndex = usize;

/// Represents a signle render target (possibly shared by multiple subpasses)
/// of a render pass.
pub type RenderPassTargetIndex = usize;

/// Specifies a predicate (boolean-valued function) on two numeric values
/// used during various kinds of tests (e.g., depth test).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CmpFn {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

// Can't define `mopo!`s in the same module as those traits due to the name
// confliction of the unqualified name of`Result`.
mopo! { arg::ArgTableSigBuilder }
mopo! { arg::ArgSig }
mopo! { arg::RootSigBuilder }
mopo! { arg::ArgPoolBuilder }
mopo! { arg::ArgPool }
mopo! { command::CmdQueueBuilder }
mopo! { command::CmdQueue }
mopo! { command::CmdBuffer }
mopo! { command::RenderCmdEncoder }
mopo! { command::ComputeCmdEncoder }
mopo! { command::CopyCmdEncoder }
mopo! { command::CmdEncoder }
mopo! { heap::HeapBuilder }
mopo! { heap::Heap }
mopo! { pass::RenderPassBuilder }
mopo! { pass::RenderPassTarget }
mopo! { pass::RenderTargetTableBuilder }
mopo! { pipeline::ComputePipelineBuilder }
mopo! { resources::ImageBuilder }
mopo! { resources::BufferBuilder }
mopo! { sampler::SamplerBuilder }
mopo! { shader::LibraryBuilder }
mopo! { sync::BarrierBuilder }

/// The `zangfx_base` prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use device::DeviceExt;
    #[doc(no_inline)]
    pub use handles::HandleImpl;
    #[doc(no_inline)]
    pub use formats::{AsIndexFormat, FloatAsScalarFormat, IntAsScalarFormat};
}

// Import all objects
#[doc(no_inline)]
pub use handles::*;
#[doc(no_inline)]
pub use objects::*;
#[doc(no_inline)]
pub use arg::*;
#[doc(no_inline)]
pub use command::*;
#[doc(no_inline)]
pub use device::*;
#[doc(no_inline)]
pub use formats::*;
#[doc(no_inline)]
pub use heap::*;
#[doc(no_inline)]
pub use limits::*;
#[doc(no_inline)]
pub use pass::*;
#[doc(no_inline)]
pub use pipeline::*;
#[doc(no_inline)]
pub use resources::*;
#[doc(no_inline)]
pub use sampler::*;
#[doc(no_inline)]
pub use shader::*;
#[doc(no_inline)]
pub use sync::*;
