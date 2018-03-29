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

#[macro_use]
extern crate ngsenumflags;
#[macro_use]
extern crate ngsenumflags_derive;

extern crate itervalues;
#[macro_use]
extern crate itervalues_derive;

#[macro_use]
#[macro_reexport(interfaces, vtable_for, mopo)] // FIXME: deprecated in favor of Macro 2.0
extern crate query_interface;

extern crate zangfx_common as common;

// `handles` defines a macro
pub mod handles;
// `objects` defines a macro too
#[macro_use]
pub mod objects;

pub mod arg;
pub mod command;
pub mod debug;
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
pub type VertexBufferIndex = usize;
/// Represents a location of a vertex attribute consumed by a vertex shader.
pub type VertexAttrIndex = usize;

/// Represents a location of an argument table in an argument binding table.
pub type ArgTableIndex = usize;
/// Represents an argument location in an argument table.
pub type ArgIndex = usize;
/// Represents an element of an array of descriptors.
pub type ArgArrayIndex = usize;

/// Represents a single render target (possibly shared by multiple subpasses)
/// of a render pass.
pub type RenderPassTargetIndex = usize;

/// Represents a single color render target of a render subpass.
pub type RenderSubpassColorTargetIndex = usize;

/// Specifies a viewport in a render pipeline.
pub type ViewportIndex = usize;

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

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    /// The X coordinate of the viewport's upper left corner.
    pub x: f32,
    /// The Y coordinate of the viewport's upper left corner.
    pub y: f32,
    /// The width of the viewport (measure in pixels).
    pub width: f32,
    /// The height of the viewport (measure in pixels).
    pub height: f32,
    /// The lower bound of the viewport's depth range.
    pub min_depth: f32,
    /// The upper bound of the viewport's depth range.
    pub max_depth: f32,
}

define_object! { arg::ArgTableSigBuilder }
define_object! { arg::ArgSig }
define_object! { arg::RootSigBuilder }
define_object! { arg::ArgPoolBuilder }
define_object! { arg::ArgPool }
define_object! { command::CmdQueueBuilder }
define_object! { command::CmdQueue }
define_object! { command::CmdBuffer }
define_object! { command::CmdPool }
define_object! { command::RenderCmdEncoder }
define_object! { command::ComputeCmdEncoder }
define_object! { command::CopyCmdEncoder }
define_object! { command::CmdEncoder }
define_object! { device::Device }
define_object! { heap::DynamicHeapBuilder }
define_object! { heap::DedicatedHeapBuilder }
define_object! { heap::Heap }
define_object! { pass::RenderPassBuilder }
define_object! { pass::RenderPassTarget }
define_object! { pass::RenderTargetTableBuilder }
define_object! { pipeline::ComputePipelineBuilder }
define_object! { pipeline::RenderPipelineBuilder }
define_object! { resources::ImageBuilder }
define_object! { resources::BufferBuilder }
define_object! { sampler::SamplerBuilder }
define_object! { shader::LibraryBuilder }
define_object! { sync::BarrierBuilder }

/// The `zangfx_base` prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use command::CmdPoolExt;
    #[doc(no_inline)]
    pub use device::DeviceExt;
    #[doc(no_inline)]
    pub use handles::HandleImpl;
    #[doc(no_inline)]
    pub use formats::{AsIndexFormat, FloatAsImageFormat, FloatAsScalarFormat, IntAsImageFormat,
                      IntAsScalarFormat};
    #[doc(no_inline)]
    pub use pipeline::RasterizerExt;
    #[doc(no_inline)]
    pub use debug::Label;
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
#[doc(no_inline)]
pub use debug::*;

#[doc(no_inline)]
pub use common::{Rect2D, Result};
