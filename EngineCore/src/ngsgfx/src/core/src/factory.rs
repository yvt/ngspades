//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::fmt::Debug;
use std::any::Any;

use super::{Backend, Result, MemoryRequirements};
use super::{RenderPassDescription, FramebufferDescription, SpecializedHeapDescription,
            ImageDescription, BufferDescription, ImageViewDescription,
            GraphicsPipelineDescription, DescriptorPoolDescription,
            DescriptorSetLayoutDescription, SamplerDescription, PipelineLayoutDescription,
            ShaderModuleDescription, ComputePipelineDescription, StencilStateDescription,
            EventDescription};

pub trait Factory<B: Backend>: Debug + Any {
    fn make_event(&self, descriptor: &EventDescription) -> Result<B::Event>;

    fn make_render_pass(&self, description: &RenderPassDescription) -> Result<B::RenderPass>;
    fn make_framebuffer(
        &self,
        description: &FramebufferDescription<B::RenderPass, B::ImageView>,
    ) -> Result<B::Framebuffer>;

    fn make_specialized_heap(
        &self,
        description: &SpecializedHeapDescription,
    ) -> Result<B::SpecializedHeap>;
    fn make_universal_heap(&self) -> Result<B::UniversalHeap>;
    fn make_image_view(&self, description: &ImageViewDescription<B::Image>)
        -> Result<B::ImageView>;

    /// Retrieve the memory requirements for a given buffer description.
    ///
    /// See the module-level documentation of [`heap`] for more about
    /// the memory requirements.
    ///
    /// Warning: The required size may be larger than `BufferDescription::size`.
    ///
    /// [`heap`]: ../heap/index.html
    /// [`SpecializedHeapUsage`]: ../heap/enum.SpecializedHeapUsage.html
    fn get_buffer_memory_requirements(&self, description: &BufferDescription)
        -> MemoryRequirements;

    /// Retrieve the memory requirements for a given image description.
    ///
    /// See the module-level documentation of [`heap`] for more about
    /// the memory requirements.
    ///
    /// [`heap`]: ../heap/index.html
    /// [`SpecializedHeapUsage`]: ../heap/enum.SpecializedHeapUsage.html
    fn get_image_memory_requirements(&self, description: &ImageDescription) -> MemoryRequirements;

    fn make_sampler(&self, description: &SamplerDescription) -> Result<B::Sampler>;

    fn make_shader_module(&self, description: &ShaderModuleDescription) -> Result<B::ShaderModule>;

    fn make_compute_pipeline(
        &self,
        description: &ComputePipelineDescription<B::PipelineLayout, B::ShaderModule>,
    ) -> Result<B::ComputePipeline>;

    fn make_graphics_pipeline(
        &self,
        description: &GraphicsPipelineDescription<B::RenderPass, B::PipelineLayout, B::ShaderModule>,
    ) -> Result<B::GraphicsPipeline>;

    fn make_stencil_state(
        &self,
        description: &StencilStateDescription<B::GraphicsPipeline>,
    ) -> Result<B::StencilState>;

    fn make_descriptor_set_layout(
        &self,
        description: &DescriptorSetLayoutDescription<B::Sampler>,
    ) -> Result<B::DescriptorSetLayout>;
    fn make_pipeline_layout(
        &self,
        description: &PipelineLayoutDescription<B::DescriptorSetLayout>,
    ) -> Result<B::PipelineLayout>;

    fn make_descriptor_pool(
        &self,
        description: &DescriptorPoolDescription,
    ) -> Result<B::DescriptorPool>;
}
