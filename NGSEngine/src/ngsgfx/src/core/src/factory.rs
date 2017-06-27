//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::fmt::Debug;
use std::any::Any;

use super::{Backend, Result, MemoryRequirements};
use super::{RenderPassDescription, FramebufferDescription, HeapDescription, ImageDescription,
            BufferDescription, ImageViewDescription, GraphicsPipelineDescription,
            DescriptorPoolDescription, DescriptorSetDescription, DescriptorSetLayoutDescription,
            SamplerDescription, PipelineLayoutDescription, ShaderModuleDescription,
            ComputePipelineDescription, StencilStateDescription,
            FenceDescription};

pub trait Factory<B: Backend>: Debug + Any {
    fn make_fence(&self, descriptor: &FenceDescription) -> Result<B::Fence>;

    fn make_render_pass(&self, description: &RenderPassDescription) -> Result<B::RenderPass>;
    fn make_framebuffer(
        &self,
        description: &FramebufferDescription<B::RenderPass, B::ImageView>,
    ) -> Result<B::Framebuffer>;

    fn make_heap(&self, description: &HeapDescription) -> Result<B::Heap>;
    fn make_image_view(&self, description: &ImageViewDescription<B::Image>)
        -> Result<B::ImageView>;
    fn get_buffer_memory_requirements(&self, description: &BufferDescription)
        -> MemoryRequirements;
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
