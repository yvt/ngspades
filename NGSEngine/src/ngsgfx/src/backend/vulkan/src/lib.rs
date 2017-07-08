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
extern crate ngsgfx_core as core;
extern crate cgmath;
pub extern crate ash;

#[macro_use]
mod macros;

mod buffer;
mod command;
mod descriptor;
mod device;
mod extif;
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

pub use extif::*;
use utils::*;

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
    pub use super::limits::*;
    pub use super::pipeline::*;
    pub use super::renderpass::*;
    pub use super::sampler::*;
    pub use super::shader::*;
    pub use super::sync::*;

    pub type GraphicsPipelineDescription<'a, T> =
        core::GraphicsPipelineDescription<'a, RenderPass<T>, PipelineLayout<T>, ShaderModule<T>>;

    pub type ComputePipelineDescription<'a, T> =
        core::ComputePipelineDescription<'a, PipelineLayout<T>, ShaderModule<T>>;

    pub type StencilStateDescription<'a, T> = core::StencilStateDescription<
        'a,
        GraphicsPipeline<T>,
    >;

    pub struct Backend<T: ::DeviceRef>(PhantomData<T>);
    impl<T: ::DeviceRef> core::Backend for Backend<T> {
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
        type UniversalHeap = Heap<T>;
        type SpecializedHeap = Heap<T>;
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

pub use self::imp::Backend;
pub use self::imp::Device;
