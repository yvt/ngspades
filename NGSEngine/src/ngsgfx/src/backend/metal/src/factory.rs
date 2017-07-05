//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core::{self, Validate};
use metal;
use std::sync::Arc;

use imp::{self, Backend, ComputePipeline, DescriptorPool, DescriptorSetLayout, Event, Framebuffer,
          GraphicsPipeline, Heap, Image, ImageView, PipelineLayout, RenderPass, Sampler,
          ShaderModule, StencilState, DeviceData};

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
    fn make_event(&self, description: &core::EventDescription) -> core::Result<Event> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(Event::new(description))
    }
    fn make_render_pass(
        &self,
        description: &core::RenderPassDescription,
    ) -> core::Result<RenderPass> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(RenderPass::new(description))
    }
    fn make_framebuffer(
        &self,
        description: &core::FramebufferDescription<RenderPass, ImageView>,
    ) -> core::Result<Framebuffer> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(Framebuffer::new(description))
    }

    fn make_specialized_heap(
        &self,
        description: &core::SpecializedHeapDescription,
    ) -> core::Result<Heap> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(Heap::new_specialized(&self.device_data, description))
    }

    fn make_universal_heap(&self) -> core::Result<Heap> {
        Ok(Heap::new_universal(&self.device_data))
    }

    fn make_image_view(
        &self,
        description: &core::ImageViewDescription<Image>,
    ) -> core::Result<ImageView> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        unimplemented!()
    }
    fn get_buffer_memory_requirements(
        &self,
        description: &core::BufferDescription,
    ) -> core::MemoryRequirements {
        // Return a dummy value since we do not have a real
        // heap implementation
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        core::MemoryRequirements {
            size: description.size,
            alignment: 1,
        }
    }
    fn get_image_memory_requirements(
        &self,
        description: &core::ImageDescription,
    ) -> core::MemoryRequirements {
        // Return a dummy value since we do not have a real
        // heap implementation
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        unimplemented!()
    }

    fn make_sampler(&self, description: &core::SamplerDescription) -> core::Result<Sampler> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Sampler::new(self.metal_device(), description)
    }

    fn make_shader_module(
        &self,
        description: &core::ShaderModuleDescription,
    ) -> core::Result<ShaderModule> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(ShaderModule::new(description))
    }

    fn make_compute_pipeline(
        &self,
        description: &core::ComputePipelineDescription<PipelineLayout, ShaderModule>,
    ) -> core::Result<ComputePipeline> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        ComputePipeline::new(self.metal_device(), description)
    }

    fn make_graphics_pipeline(
        &self,
        description: &imp::GraphicsPipelineDescription,
    ) -> core::Result<GraphicsPipeline> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        GraphicsPipeline::new(self.metal_device(), description)
    }

    fn make_stencil_state(
        &self,
        description: &core::StencilStateDescription<GraphicsPipeline>,
    ) -> core::Result<StencilState> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        StencilState::new(self.metal_device(), description)
    }

    fn make_descriptor_set_layout(
        &self,
        description: &core::DescriptorSetLayoutDescription<Sampler>,
    ) -> core::Result<DescriptorSetLayout> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        DescriptorSetLayout::new(description)
    }
    fn make_pipeline_layout(
        &self,
        description: &core::PipelineLayoutDescription<DescriptorSetLayout>,
    ) -> core::Result<PipelineLayout> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        PipelineLayout::new(description)
    }

    fn make_descriptor_pool(
        &self,
        description: &core::DescriptorPoolDescription,
    ) -> core::Result<DescriptorPool> {
        description.debug_expect_valid(Some(self.device_data.capabilities()), "");
        Ok(DescriptorPool::new(&self.device_data, description))
    }
}
