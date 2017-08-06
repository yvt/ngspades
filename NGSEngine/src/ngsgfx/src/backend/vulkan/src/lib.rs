//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Vulkan Backend
//! ====================
//!
//! Implements a NgsGFX backend using the Vulkan API.
//!
//! Feature Mappings
//! ----------------
//!
//! TODO
//!
//! ### Debugging
//!
//! Some debug layers are proided by the LunarG Vulkan SDK. Some of them can be
//! enabled via trait methods implemented by [`InstanceBuilder`], provided that they
//! are installed on the target system. They are no-op if the corresponding layers
//! or extensions are not installed.
//!
//!  - Standard validation layers (`VK_LAYER_LUNARG_standard_validation`)
//!    are enabled by calling [`enable_validation`].
//!  - Validation messages are delivered via the `VK_EXT_debug_report`
//!    extension. An application can register one or more debug report handlers
//!    by calling [`enable_debug_report`].
//!  - The `VK_EXT_debug_marker` extension provides the ability to give objects
//!    names (via the `Marker` trait) and to insert debug markers into command
//!    buffers to ease the inspection by an external debugger (e.g., RenderDoc).
//!    This extension can be enabled by calling [`enable_debug_marker`].
//!
//!    TODO: ... which is not supported because `ash` does not have a wrapper for it yet
//!
//! [`InstanceBuilder`]: struct.InstanceBuilder.html
//! [`enable_validation`]: struct.InstanceBuilder.html#method.enable_validation
//! [`enable_debug_report`]: struct.InstanceBuilder.html#method.enable_debug_report
//! [`enable_debug_marker`]: struct.InstanceBuilder.html#method.enable_debug_marker
//!
//! Performance Notes
//! -----------------
//!
//! ### Inter-queue Fences
//!
//! When creating `Fence`, make sure to specify only engines that are actually
//! going to wait on the fence as `wait_engines`.
//!
//! Inter-queue fence synchronization is implemented using `vk::Semaphore` and
//! `vk::Semaphore` is created for every destination engine with a distinct device
//! queue. Specifying some engine in `wait_engines` and not waiting on the fence
//! from that engine might leave a `vk::Semaphore` signaled but no batch waiting
//! for it. Updating such fences will be penalized because that `vk::Semaphore`
//! must be waited first before signalling it again.
//!
#![feature(optin_builtin_traits)]
extern crate ngsgfx_core as core;
extern crate ngsgfx_common;
extern crate cgmath;
extern crate smallvec;
extern crate atomic_refcell;
extern crate parking_lot;
pub extern crate ash;

#[macro_use]
mod macros;

mod buffer;
mod command;
mod debug;
mod descriptor;
mod device;
mod device_ll;
mod extif;
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
mod utils;

pub use extif::*;
use utils::*;
use ngsgfx_common::refeq::{RefEqBox, RefEqArc};

/// Low-level wrappers.
pub mod ll {
    pub use super::device_ll::*;
}

/// Implementations of NgsGFX primitives.
pub mod imp {
    use core;
    use std::marker::PhantomData;

    pub use super::buffer::*;
    pub use super::command::*;
    pub use super::debug::*;
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
    pub use super::utils::*;

    pub type GraphicsPipelineDescription<'a, T> =
        core::GraphicsPipelineDescription<'a, RenderPass<T>, PipelineLayout<T>, ShaderModule<T>>;

    pub type ComputePipelineDescription<'a, T> =
        core::ComputePipelineDescription<'a, PipelineLayout<T>, ShaderModule<T>>;

    pub type StencilStateDescription<'a, T> = core::StencilStateDescription<
        'a,
        GraphicsPipeline<T>,
    >;

    pub type FramebufferDescription<'a, T> = core::FramebufferDescription<
        'a,
        RenderPass<T>,
        ImageView<T>,
    >;

    pub struct ManagedEnvironment;
    impl core::Environment for ManagedEnvironment {
        type Backend = ManagedBackend;

        type DeviceBuilder = DeviceBuilder;
        type Instance = Instance;
        type InstanceBuilder = InstanceBuilder;
    }

    pub type ManagedBackend = Backend<super::ManagedDeviceRef>;
    pub struct Backend<T: ::DeviceRef>(PhantomData<T>);
    impl<T: super::DeviceRef> core::Backend for Backend<T> {
        type Buffer = Buffer<T>;
        type CommandBuffer = CommandBuffer<T>;
        type CommandQueue = CommandQueue<T>;
        type ComputePipeline = ComputePipeline<T>;
        type DescriptorPool = DescriptorPool<T>;
        type DescriptorSet = DescriptorSet<T>;
        type DescriptorSetLayout = DescriptorSetLayout<T>;
        type Device = Device<T>;
        type DeviceCapabilities = DeviceCapabilities;
        type Factory = Device<T>;
        type Fence = Fence<T>;
        type Event = Event<T>;
        type Framebuffer = Framebuffer<T>;
        type GraphicsPipeline = GraphicsPipeline<T>;
        type UniversalHeap = UniversalHeap<T>;
        type SpecializedHeap = SpecializedHeap<T>;
        type Image = Image<T>;
        type ImageView = ImageView<T>;
        type PipelineLayout = PipelineLayout<T>;
        type RenderPass = RenderPass<T>;
        type Sampler = Sampler<T>;
        type SecondaryCommandBuffer = SecondaryCommandBuffer<T>;
        type StencilState = StencilState<T>;
        type ShaderModule = ShaderModule<T>;
    }

}

pub use self::imp::{Backend, ManagedEnvironment, ManagedBackend};
pub use self::imp::{Device, DeviceBuilder, Instance, InstanceBuilder};
