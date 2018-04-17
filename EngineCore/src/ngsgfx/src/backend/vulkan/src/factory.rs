//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core::{self, Validate};

use {Backend, DeviceRef};
use imp::{self, ComputePipeline, DescriptorPool, Device, DescriptorSetLayout, Event, Framebuffer,
          GraphicsPipeline, Image, ImageView, PipelineLayout, RenderPass, Sampler, ShaderModule,
          StencilState, UnassociatedImage, UnassociatedBuffer, UniversalHeap, SpecializedHeap};

impl<T: DeviceRef> core::Factory<Backend<T>> for Device<T> {
    fn make_event(&self, description: &core::EventDescription) -> core::Result<Event<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");

        self.data.make_event(description)
    }
    fn make_render_pass(
        &self,
        description: &core::RenderPassDescription,
    ) -> core::Result<RenderPass<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        RenderPass::new(self.device_ref(), description)
    }
    fn make_framebuffer(
        &self,
        description: &core::FramebufferDescription<RenderPass<T>, ImageView<T>>,
    ) -> core::Result<Framebuffer<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        Framebuffer::new(description)
    }

    fn make_specialized_heap(
        &self,
        description: &core::SpecializedHeapDescription,
    ) -> core::Result<SpecializedHeap<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        SpecializedHeap::new(self.data.clone(), description)
    }

    fn make_universal_heap(&self) -> core::Result<UniversalHeap<T>> {
        Ok(UniversalHeap::new(self.data.clone()))
    }

    fn make_image_view(
        &self,
        description: &core::ImageViewDescription<Image<T>>,
    ) -> core::Result<ImageView<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        ImageView::new(self.device_ref(), description, self.capabilities())
    }
    fn get_buffer_memory_requirements(
        &self,
        description: &core::BufferDescription,
    ) -> core::MemoryRequirements {
        // TODO: make `get_buffer_memory_requirements` return `Result<_>`
        let proto = UnassociatedBuffer::new(self.device_ref(), description).unwrap();
        let req = proto.memory_requirements();
        core::MemoryRequirements {
            size: req.size,
            alignment: req.alignment,
        }
    }
    fn get_image_memory_requirements(
        &self,
        description: &core::ImageDescription,
    ) -> core::MemoryRequirements {
        // TODO: make `get_image_memory_requirements` return `Result<_>`
        let proto = UnassociatedImage::new(self.device_ref(), description).unwrap();
        let req = proto.memory_requirements();
        core::MemoryRequirements {
            size: req.size,
            alignment: req.alignment,
        }
    }

    fn make_sampler(&self, description: &core::SamplerDescription) -> core::Result<Sampler<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        Sampler::new(self.device_ref(), description)
    }

    fn make_shader_module(
        &self,
        description: &core::ShaderModuleDescription,
    ) -> core::Result<ShaderModule<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        ShaderModule::new(self.device_ref(), description)
    }

    fn make_compute_pipeline(
        &self,
        description: &imp::ComputePipelineDescription<T>,
    ) -> core::Result<ComputePipeline<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        ComputePipeline::new(self.device_ref(), description)
    }

    fn make_graphics_pipeline(
        &self,
        description: &imp::GraphicsPipelineDescription<T>,
    ) -> core::Result<GraphicsPipeline<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        GraphicsPipeline::new(self.device_ref(), description)
    }

    fn make_stencil_state(
        &self,
        description: &core::StencilStateDescription<GraphicsPipeline<T>>,
    ) -> core::Result<StencilState> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        Ok(StencilState::new(description))
    }

    fn make_descriptor_set_layout(
        &self,
        description: &core::DescriptorSetLayoutDescription<Sampler<T>>,
    ) -> core::Result<DescriptorSetLayout<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        DescriptorSetLayout::new(self.device_ref(), description)
    }
    fn make_pipeline_layout(
        &self,
        description: &core::PipelineLayoutDescription<DescriptorSetLayout<T>>,
    ) -> core::Result<PipelineLayout<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        PipelineLayout::new(self.device_ref(), description)
    }

    fn make_descriptor_pool(
        &self,
        description: &core::DescriptorPoolDescription,
    ) -> core::Result<DescriptorPool<T>> {
        description.debug_expect_valid(Some(self.capabilities()), "");
        DescriptorPool::new(self.device_ref(), description)
    }
}