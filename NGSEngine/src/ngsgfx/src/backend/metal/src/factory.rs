//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core::{self, Validate};
use metal;
use std::sync::Arc;

use ref_hash;
use imp::{Backend, Buffer, BufferView, ComputePipeline, DescriptorPool, DescriptorSet,
          DescriptorSetLayout, Fence, Framebuffer, GraphicsPipeline, Heap, Image, ImageView,
          PipelineLayout, RenderPass, Sampler, Semaphore, ShaderModule, StencilState, DeviceData};

#[derive(Debug)]
pub struct Factory {
    device_data: Arc<DeviceData>,
}

impl Factory {
    pub(crate) fn new(device_data: Arc<DeviceData>) -> Self {
        Self { device_data: device_data }
    }

    fn metal_device(&self) -> metal::MTLDevice {
        self.device_data.metal_device()
    }
}

impl core::Factory<Backend> for Factory {
    fn make_fence(&self, description: &core::FenceDescription) -> core::Result<Fence> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(Fence::new(description))
    }
    fn make_semaphore(&self, description: &core::SemaphoreDescription) -> core::Result<Semaphore> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(Semaphore::new(description))
    }
    fn make_render_pass(&self, description: &core::RenderPassDescription) -> core::Result<RenderPass> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(RenderPass::new(description))
    }
    fn make_framebuffer(&self,
                        description: &core::FramebufferDescription<RenderPass, ImageView>)
                        -> core::Result<Framebuffer> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(Framebuffer::new(description))
    }

    fn make_heap(&self, description: &core::HeapDescription) -> core::Result<Heap> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(Heap::new(&self.device_data, description))
    }

    /// Creates a buffer view.
    fn make_buffer_view(&self,
                        description: &core::BufferViewDescription<Buffer>)
                        -> core::Result<BufferView> {
        unimplemented!()
    }
    fn make_image_view(&self,
                       description: &core::ImageViewDescription<Image>)
                       -> core::Result<ImageView> {
        unimplemented!()
    }
    fn get_buffer_memory_requirements(&self,
                                      description: &core::BufferDescription)
                                      -> core::MemoryRequirements {
        // Return a dummy value since we do not have a real
        // heap implementation
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        core::MemoryRequirements {
            size: description.size,
            alignment: 1,
        }
    }
    fn get_image_memory_requirements(&self, description: &core::ImageDescription) -> core::MemoryRequirements {
        // Return a dummy value since we do not have a real
        // heap implementation
        unimplemented!()
    }

    fn make_sampler(&self, description: &core::SamplerDescription) -> core::Result<Sampler> {
        Sampler::new(self.metal_device(), description)
    }

    fn make_shader_module(&self, description: &core::ShaderModuleDescription) -> core::Result<ShaderModule> {
        unimplemented!()
    }

    fn make_compute_pipeline(&self,
                             description: &core::ComputePipelineDescription<PipelineLayout,
                                                                      ShaderModule>)
                             -> core::Result<ComputePipeline> {
        unimplemented!()
    }

    fn make_graphics_pipeline(&self,
                              description: &core::GraphicsPipelineDescription<RenderPass,
                                                                        PipelineLayout,
                                                                        ShaderModule>)
                              -> core::Result<GraphicsPipeline> {
        unimplemented!()
    }

    fn make_stencil_state(&self, description: &core::StencilStateDescription) -> core::Result<StencilState> {
        unimplemented!()
    }

    fn make_descriptor_set_layout(&self,
                                  description: &core::DescriptorSetLayoutDescription<Sampler>)
                                  -> core::Result<DescriptorSetLayout> {
        DescriptorSetLayout::new(description)
    }
    fn make_pipeline_layout(&self,
                            description: &core::PipelineLayoutDescription<DescriptorSetLayout>)
                            -> core::Result<PipelineLayout> {
        PipelineLayout::new(description)
    }

    fn make_descriptor_pool(&self,
                            description: &core::DescriptorPoolDescription)
                            -> core::Result<DescriptorPool> {
        unimplemented!()
    }
}