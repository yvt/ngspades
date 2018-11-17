//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! This crate is a part of [ZanGFX](../zangfx/index.html) and provides the base
//! interface for backend implementations.
#![feature(unsize)]
#![feature(raw)]
#![feature(get_type_id)]
#![feature(pin)]
#![warn(rust_2018_idioms)]

pub use zangfx_common as common;

// Rexport macros from `query_interface`
#[doc(no_inline)]
pub use query_interface::{interfaces, mopo, vtable_for};

// `handles` defines a macro
#[macro_use]
pub mod handles;
// `objects` defines a macro too
#[macro_use]
pub mod objects;

pub mod arg;
pub mod command;
pub mod debug;
pub mod device;
pub mod error;
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

define_object! { dyn ArgTableSigBuilder }
define_object! { dyn ArgSig }
define_object! { dyn RootSigBuilder }
define_object! { dyn ArgPoolBuilder }
define_object! { dyn ArgPool }
define_object! { dyn CmdQueueBuilder }
define_object! { dyn CmdQueue }
define_object! { dyn CmdBuffer }
define_object! { dyn RenderCmdEncoder }
define_object! { dyn ComputeCmdEncoder }
define_object! { dyn CopyCmdEncoder }
define_object! { dyn CmdEncoder }
define_object! { dyn Device }
define_object! { dyn DynamicHeapBuilder }
define_object! { dyn DedicatedHeapBuilder }
define_object! { dyn Heap }
define_object! { dyn RenderPassBuilder }
define_object! { dyn RenderPassTarget }
define_object! { dyn RenderTargetTableBuilder }
define_object! { dyn ComputePipelineBuilder }
define_object! { dyn RenderPipelineBuilder }
define_object! { dyn ImageBuilder }
define_object! { dyn BufferBuilder }
define_object! { dyn SamplerBuilder }
define_object! { dyn LibraryBuilder }

/// The `zangfx_base` prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::{
        command::CmdEncoderExt,
        debug::Label,
        device::DeviceExt,
        formats::{
            AsIndexFormat, FloatAsImageFormat, FloatAsScalarFormat, IntAsImageFormat,
            IntAsScalarFormat,
        },
        handles::CloneHandle,
        pipeline::RasterizerExt,
    };
}

// Import all objects
#[doc(no_inline)]
pub use crate::{
    arg::*, command::*, debug::*, device::*, error::*, formats::*, handles::*, heap::*, limits::*,
    objects::*, pass::*, pipeline::*, resources::*, sampler::*, shader::*, sync::*,
};

#[doc(no_inline)]
pub use zangfx_common::Rect2D;
