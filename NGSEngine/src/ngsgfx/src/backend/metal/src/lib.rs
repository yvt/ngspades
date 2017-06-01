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
//!    use of `MTLHeap` are provided.
//!
//!    `MTLBuffer` provides a method that allows users to sub-allocate its portion to create
//!    texture views, but this is inappropriate for a general use since it is not designed for
//!    performance but rather a way to create linearly layouted textures.
//!  - **`Semaphore`** - `MTLFence` might seem to be the counterpart, but this is not true because
//!    `MTLFence` does not support inter-queue synchronization, which has to be maintained by
//!    the software.
//!  - **`Fence`** - Mapped to callbacks from `MTLCommandBuffer`.
//!  - **`BufferView`** - No direct Metal counterparts are available. TODO: emulate by 2D texture?
//!  - **`CommandBuffer`** - Maps naturally to Metal except that one `CommandBuffer` can have multiple
//!    subpasses, each of which is mapped to a `MTLRenderPassDescriptor`.
//!  - **`RenderPass`** - Each subpass is mapped to a Metal render pass. We could leverage the tile
//!    local storage by merging multiple subpasses into one, but no known macOS version supports
//!    this, so we stick to the naïve approach.
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

mod buffer;
mod command;
mod descriptor;
mod device;
mod factory;
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
    pub use super::heap::*;
    pub use super::image::*;
    pub use super::limits::*;
    pub use super::pipeline::*;
    pub use super::renderpass::*;
    pub use super::sampler::*;
    pub use super::shader::*;
    pub use super::sync::*;

    pub struct Resources {}
    impl core::Resources for Resources {
        type Buffer = Buffer;
        type BufferView = BufferView;
        type ComputePipeline = ComputePipeline;
        type DescriptorPool = DescriptorPool;
        type DescriptorSet = DescriptorSet;
        type DescriptorSetLayout = DescriptorSetLayout;
        type Fence = Fence;
        type Framebuffer = Framebuffer;
        type GraphicsPipeline = GraphicsPipeline;
        type Heap = Heap;
        type Image = Image;
        type ImageView = ImageView;
        type PipelineLayout = PipelineLayout;
        type RenderPass = RenderPass;
        type Sampler = Sampler;
        type Semaphore = Semaphore;
        type ShaderModule = ShaderModule;
        type StencilState = StencilState;
    }

}

pub use self::imp::Resources;
pub use self::imp::Device;

