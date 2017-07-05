//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Metal Backend
//! ====================
//!
//! Feature Mappings
//! ----------------
//!
//!  - **`Heap`** - A manually managed heap is available (`MTLHeap`), but is not supported
//!    on macOS (yet?). Since we are targetting desktop platforms, no implementations that make
//!    use of `MTLHeap` are provided. TODO: Metal 2 supports `MTLHeap` on both of macOS and iOS
//!
//!    `MTLBuffer` provides a method that allows users to sub-allocate its portion to create
//!    texture views, but this is inappropriate for a general use since it is not designed for
//!    performance but rather a way to create linearly layouted textures.
//!  - **`Event`** - Mapped to callbacks from `MTLCommandBuffer`.
//!  - **`BufferView`** - No direct Metal counterparts are available. TODO: emulate by 2D texture?
//!  - **`CommandBuffer`** - Maps naturally to Metal except that one `CommandBuffer` can have multiple
//!    subpasses, each of which is mapped to a `MTLRenderPassDescriptor`.
//!  - **`RenderPass`** - Each subpass is mapped to a Metal render pass. We could leverage the tile
//!    local storage by merging multiple subpasses into one, but no known macOS version supports
//!    this, so we stick to the naïve approach.
//!  - **`DescriptorSet`** - Flattened to argument tables. TODO: make use of Metal 2's
//!    indirect argument tables for more efficiency
//!
//! ### Unimplemented Features
//!
//!  - Exception handling - Out of memory will result in an undefined behavior.
//!    This is supposed to cause a panic.
//!
extern crate ngsgfx_core as core;
extern crate ngsgfx_metal_rs as metal;
extern crate enumflags;
extern crate cgmath;
extern crate atomic_refcell;
extern crate block;
extern crate spirv_cross;
extern crate rspirv;
extern crate spirv_headers;

mod buffer;
mod command;
mod descriptor;
mod device;
mod factory;
mod formats;
mod heap;
mod image;
mod limits;
mod pipeline;
mod renderpass;
mod sampler;
mod shader;
mod sync;
mod utils;

use utils::*;

/// Reexports items from `metal-rs`.
pub mod ll {
    pub use super::metal::*;
}

/// Implementations of NgsGFX primitives.
pub mod imp {
    use core;

    pub use super::buffer::*;
    pub use super::command::*;
    pub use super::descriptor::*;
    pub use super::device::*;
    pub use super::factory::*;
    pub use super::formats::*;
    pub use super::heap::*;
    pub use super::image::*;
    pub use super::limits::*;
    pub use super::pipeline::*;
    pub use super::renderpass::*;
    pub use super::sampler::*;
    pub use super::shader::*;
    pub use super::sync::*;

    pub type GraphicsPipelineDescription<'a> = core::GraphicsPipelineDescription<
        'a,
        RenderPass,
        PipelineLayout,
        ShaderModule,
    >;

    pub type ComputePipelineDescription<'a> = core::ComputePipelineDescription<
        'a,
        PipelineLayout,
        ShaderModule,
    >;

    pub type StencilStateDescription<'a> = core::StencilStateDescription<'a, GraphicsPipeline>;

    pub struct Backend {}
    impl core::Backend for Backend {
        type Buffer = Buffer;
        type CommandBuffer = CommandBuffer;
        type CommandQueue = CommandQueue;
        type ComputePipeline = ComputePipeline;
        type DescriptorPool = DescriptorPool;
        type DescriptorSet = DescriptorSet;
        type DescriptorSetLayout = DescriptorSetLayout;
        type Device = Device;
        type DeviceCapabilities = DeviceCapabilities;
        type Factory = Factory;
        type Fence = Fence;
        type Event = Event;
        type Framebuffer = Framebuffer;
        type GraphicsPipeline = GraphicsPipeline;
        type SpecializedHeap = Heap;
        type UniversalHeap = Heap;
        type Image = Image;
        type ImageView = ImageView;
        type PipelineLayout = PipelineLayout;
        type RenderPass = RenderPass;
        type Sampler = Sampler;
        type SecondaryCommandBuffer = SecondaryCommandBuffer;
        type ShaderModule = ShaderModule;
        type StencilState = StencilState;
    }

}

pub use self::imp::Backend;
pub use self::imp::Device;
