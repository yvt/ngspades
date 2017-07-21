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
#![feature(optin_builtin_traits)]
extern crate ngsgfx_core as core;
extern crate ngsgfx_common;
extern crate cgmath;
extern crate smallvec;
extern crate atomic_refcell;
pub extern crate ash;

#[macro_use]
mod macros;

mod buffer;
mod command;
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

    pub type GraphicsPipelineDescription<'a, T> =
        core::GraphicsPipelineDescription<'a, RenderPass<T>, PipelineLayout<T>, ShaderModule<T>>;

    pub type ComputePipelineDescription<'a, T> =
        core::ComputePipelineDescription<'a, PipelineLayout<T>, ShaderModule<T>>;

    pub type StencilStateDescription<'a, T> = core::StencilStateDescription<
        'a,
        GraphicsPipeline<T>,
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
