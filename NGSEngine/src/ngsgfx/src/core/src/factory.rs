//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::fmt::Debug;
use std::hash::Hash;
use std::cmp::{Eq, PartialEq};
use std::any::Any;

use super::{Resources, Result, MemoryRequirements};
use super::{RenderPassDescription, FramebufferDescription, HeapDescription, ImageDescription,
            BufferDescription, ImageViewDescription, GraphicsPipelineDescription,
            DescriptorPoolDescription, DescriptorSetDescription, DescriptorSetLayoutDescription,
            SamplerDescription, PipelineLayoutDescription, BufferViewDescription,
            ShaderModuleDescription, ComputePipelineDescription, StencilStateDescription};

pub trait Factory<R: Resources>: Hash + Debug + Eq + PartialEq + Any {
    fn make_render_pass(&self, description: &RenderPassDescription) -> Result<R::RenderPass>;
    fn make_framebuffer(&self,
                        description: &FramebufferDescription<R::RenderPass, R::ImageView>)
                        -> Result<R::Framebuffer>;

    fn make_heap(&self, description: &HeapDescription) -> Result<R::Heap>;

    /// Creates a buffer view.
    fn make_buffer_view(&self,
                        description: &BufferViewDescription<R::Buffer>)
                        -> Result<R::BufferView>;
    fn make_image_view(&self,
                       description: &ImageViewDescription<R::Image>)
                       -> Result<R::ImageView>;
    fn get_buffer_memory_requirements(&self,
                                      description: &BufferDescription)
                                      -> MemoryRequirements;
    fn get_image_memory_requirements(&self, description: &ImageDescription) -> MemoryRequirements;

    fn make_sampler(&self, description: &SamplerDescription) -> Result<R::Sampler>;

    fn make_shader_module(&self, description: &ShaderModuleDescription) -> Result<R::ShaderModule>;

    fn make_compute_pipeline(&self,
                             description: &ComputePipelineDescription<R::PipelineLayout,
                                                                      R::ShaderModule>)
                             -> Result<R::ComputePipeline>;

    fn make_graphics_pipeline(&self,
                              description: &GraphicsPipelineDescription<R::RenderPass,
                                                                        R::PipelineLayout,
                                                                        R::ShaderModule>)
                              -> Result<R::GraphicsPipeline>;

    fn make_stencil_state(&self, description: &StencilStateDescription) -> Result<R::StencilState>;

    fn make_descriptor_set_layout(&self,
                                  description: &DescriptorSetLayoutDescription<R::Sampler>)
                                  -> Result<R::DescriptorSetLayout>;
    fn make_pipeline_layout(&self,
                            description: &PipelineLayoutDescription<R::DescriptorSetLayout>)
                            -> Result<R::PipelineLayout>;

    fn make_descriptor_pool(&self,
                            description: &DescriptorPoolDescription)
                            -> Result<R::DescriptorPool>;
    fn make_descriptor_sets(&self,
                            description: &DescriptorSetDescription<R::DescriptorSetLayout>,
                            pool: &R::DescriptorPool)
                            -> Result<R::DescriptorSet>;
}
