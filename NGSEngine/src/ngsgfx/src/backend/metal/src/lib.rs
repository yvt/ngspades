//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Metal Backend
//! ====================
//!
//! Implements a NgsGFX backend using Apple's Metal API.
//!
//! Metal is one of the primary target APIs of NgsGFX. This is why large portions
//! of NgsGFX's API resemble that of Metal.
//!
//! Feature Mappings
//! ----------------
//!
//!  - **`UniversalHeap`** - Naturally mapped to non-heap resource allocations.
//!  - **`SpecializedHeap`** - Unavailable. The dummy implementation behaves
//!    mostly like `UniversalHeap`.
//!
//!    Metal 2 that comes with macOS 10.13 introduces `MTLHeap`.
//!    `SpecializedHeap` might be implemented using `MTLHeap` eventually.
//!  - **`Event`** - Mapped to callbacks from `MTLCommandBuffer`.
//!  - **`Fence`** - Currently no-op. Will be mapped to `MTLFence` eventually.
//!  - **`CommandBuffer`** - Naturally mapped to `MTLCommandBuffer`.
//!    The concept of `DeviceEngine` does not exist in Metal, so it is just ignored.
//!  - **`RenderPass`** - Each subpass is mapped to a Metal render pass. We could leverage the tile
//!    local storage by merging multiple subpasses into one, but no known macOS version supports
//!    this, so we stick to the naïve approach.
//!  - **`DescriptorSet`** - Flattened to argument tables.
//!
//! ### Unimplemented Features
//!
//! The following to-do list is not comprehensive.
//!
//!  - Exception handling - Out of device/host memory will result in an undefined behavior.
//!    This is supposed to return a `Err` value or cause a panic.
//!  - Device lost handling - I just do not know what will go wrong.
//!  - Some shader instructions - SPIR-V shaders are translated into MSL using the modified version of SPIRV-Cross,
//!    but not the entire set of instructions are supported. For example, memory barrier instructions
//!    are currently ignored because the direct counterparts do not exist in MSL. Memory barriers are crucial
//!    in compute shaders, so control barrier instructions were modified to generate memory barriers as well.
//!    This work-around might be removed in the future if we could find a way to implement memory barriers correctly.
//!    To ensure the future compatibility, use the following GLSL snippet to place a work-group memory/control
//!    barrier:
//!
//!    ```glsl
//!    groupMemoryBarrier();    // OpMemoryBarrier - no MSL counterparts available
//!    barrier();               // OpControlBarrier - mapped to workgroup_barrier(...)
//!    ```
//!
extern crate ngsgfx_core as core;
extern crate ngsgfx_metal_rs as metal;
extern crate cgmath;
extern crate atomic_refcell;
extern crate block;
extern crate spirv_cross;
extern crate rspirv;
extern crate spirv_headers;
extern crate cocoa;

mod buffer;
mod command;
mod descriptor;
mod device;
mod factory;
mod formats;
mod heap;
mod image;
mod instance;
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
    pub use super::instance::*;
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

    pub struct Environment;
    impl core::Environment for Environment {
        type Backend = Backend;

        type DeviceBuilder = DeviceBuilder;
        type Instance = Instance;
        type InstanceBuilder = InstanceBuilder;
    }

    pub struct Backend;
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

pub use self::imp::{Backend, Device, Environment, DeviceBuilder, Instance, InstanceBuilder};
