//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngsgfx_core as core;
extern crate enumflags;
extern crate cgmath;

use std::time::Duration;
use enumflags::BitFlags;
use cgmath::Vector3;

pub struct Resources {}

impl core::Resources for Resources {
    type Buffer = Buffer;
    type BufferView = BufferView;
    type ComputePipeline = ComputePipeline;
    type DescriptorPool = DescriptorPool;
    type DescriptorSet = DescriptorSet;
    type DescriptorSetLayout = DescriptorSetLayout;
    type Fence = Fence;
    type Framebuffer = Framebuffer;
    type GraphicsPipeline = GraphicsPipeline;
    type Heap = Heap;
    type Image = Image;
    type ImageView = ImageView;
    type PipelineLayout = PipelineLayout;
    type RenderPass = RenderPass;
    type Sampler = Sampler;
    type Semaphore = Semaphore;
    type ShaderModule = ShaderModule;
    type StencilState = StencilState;
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct CommandBuffer {}

impl core::CommandBuffer<Resources> for CommandBuffer {
    type GraphicsCommandEncoder = GraphicsCommandEncoder;
    type ComputeCommandEncoder = ComputeCommandEncoder;
    type BlitCommandEncoder = BlitCommandEncoder;

    fn reset(&mut self) { unimplemented!() }

    fn state(&self) -> core::CommandBufferState { unimplemented!() }
    fn wait_completion(&self, timeout: Duration) -> core::Result<bool> { unimplemented!() }

    fn graphics_command_encoder(&mut self,
                                description: &core::GraphicsCommandEncoderDescription<RenderPass,
                                                                                      Framebuffer>)
                                -> &mut Self::GraphicsCommandEncoder { unimplemented!() }
    fn compute_command_encoder(&mut self) -> &mut Self::ComputeCommandEncoder { unimplemented!() }
    fn blit_command_encoder(&mut self) -> &mut Self::BlitCommandEncoder { unimplemented!() }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct GraphicsCommandEncoder {}

impl core::CommandEncoder<Resources> for GraphicsCommandEncoder {
    fn wait_semaphore(&mut self,
                      semaphore: &Semaphore,
                      stage_mask: BitFlags<core::PipelineStageFlags>) { unimplemented!() }
    fn signal_semaphore(&mut self, semaphore: &Semaphore) { unimplemented!() }
    fn end_encoding(&mut self) { unimplemented!() }
}
impl core::GraphicsCommandEncoder<Resources> for GraphicsCommandEncoder {
    fn next_subpass(&mut self) { unimplemented!() }
    fn bind_graphics_pipeline(&mut self, pipeline: &GraphicsPipeline) { unimplemented!() }
    fn set_blend_constants(&mut self, value: &[f32; 4]) { unimplemented!() }
    fn set_depth_bias(&mut self, value: &Option<core::DepthBias>) { unimplemented!() }
    fn set_depth_bounds(&mut self, value: &Option<core::DepthBounds>) { unimplemented!() }
    fn set_stencil_state(&mut self, value: &StencilState) { unimplemented!() }
    fn set_viewport(&mut self, value: &core::Viewport) { unimplemented!() }
    fn set_scissor_rect(&mut self, value: &core::Rect2D<i32>) { unimplemented!() }
    fn bind_descriptor_sets(&mut self, pipeline_layout: &PipelineLayout,
        start_index: usize, descriptor_sets: &[DescriptorSet],
        dynamic_offsets: &[u32]) { unimplemented!() }
    fn draw(&mut self,
            num_vertices: u32,
            num_instances: u32,
            start_vertex_index: u32,
            start_instance_index: u32) { unimplemented!() }
    fn draw_indexed(&mut self,
                    num_vertices: u32,
                    num_instances: u32,
                    start_vertex_index: u32,
                    index_offset: u32,
                    start_instance_index: u32) { unimplemented!() }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ComputeCommandEncoder {}

impl core::CommandEncoder<Resources> for ComputeCommandEncoder {
    fn wait_semaphore(&mut self,
                      semaphore: &Semaphore,
                      stage_mask: BitFlags<core::PipelineStageFlags>) { unimplemented!() }
    fn signal_semaphore(&mut self, semaphore: &Semaphore) { unimplemented!() }
    fn end_encoding(&mut self) { unimplemented!() }
}
impl core::ComputeCommandEncoder<Resources> for ComputeCommandEncoder {
    fn bind_compute_pipeline(&mut self, pipeline: &ComputePipeline) { unimplemented!() }

    fn dispatch(&mut self, workgroup_count: Vector3<u32>) { unimplemented!() }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct BlitCommandEncoder {}

impl core::CommandEncoder<Resources> for BlitCommandEncoder {
    fn wait_semaphore(&mut self,
                      semaphore: &Semaphore,
                      stage_mask: BitFlags<core::PipelineStageFlags>) { unimplemented!() }
    fn signal_semaphore(&mut self, semaphore: &Semaphore) { unimplemented!() }
    fn end_encoding(&mut self) { unimplemented!() }
}
impl core::BlitCommandEncoder<Resources> for BlitCommandEncoder {
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct RenderPass {}

impl core::RenderPass for RenderPass {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ImageView {}

impl core::ImageView for ImageView {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Image {}

impl core::Image for Image {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Buffer {}

impl core::Buffer for Buffer {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Heap {}

impl core::Heap<Resources> for Heap {
    fn make_buffer(&mut self, description: &core::BufferDescription) -> core::Result<Option<(Self::Allocation, Buffer)>> { unimplemented!() }
    fn make_image(&mut self, description: &core::ImageDescription) -> core::Result<Option<(Self::Allocation, Image)>> { unimplemented!() }
}

impl core::MappableHeap for Heap {
    type Allocation = ();
    type MappingInfo = ();
    fn make_aliasable(&mut self, allocation: &mut Self::Allocation) { unimplemented!() }
    fn deallocate(&mut self, allocation: &mut Self::Allocation) { unimplemented!() }
    unsafe fn raw_unmap_memory(&mut self, info: Self::MappingInfo) { unimplemented!() }
    unsafe fn raw_map_memory(&mut self, allocation: &mut Self::Allocation) -> (*mut u8, usize, Self::MappingInfo) { unimplemented!() }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Framebuffer {}

impl core::Framebuffer for Framebuffer {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct GraphicsPipeline {}

impl core::GraphicsPipeline for GraphicsPipeline {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ComputePipeline {}

impl core::ComputePipeline for ComputePipeline {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct DescriptorPool {}

impl core::DescriptorPool for DescriptorPool {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct DescriptorSet {}

impl core::DescriptorSet<Resources> for DescriptorSet {
    fn update(&self, writes: &[core::WriteDescriptorSet<Resources>]) {
        unimplemented!()
    }
    fn copy_from(&self, copies: &[core::CopyDescriptorSet<Self>]) {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct DescriptorSetLayout {}

impl core::DescriptorSetLayout for DescriptorSetLayout {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PipelineLayout {}

impl core::PipelineLayout for PipelineLayout {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Fence {}

impl core::Fence for Fence {
    fn reset(&self) -> core::Result<()> {
        unimplemented!()
    }
    fn wait(&self, timeout: Duration) -> core::Result<bool> {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Semaphore {}

impl core::Semaphore for Semaphore {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Sampler {}

impl core::Sampler for Sampler {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct BufferView {}

impl core::BufferView for BufferView {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ShaderModule {}

impl core::ShaderModule for ShaderModule {

}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct StencilState {}

impl core::StencilState for StencilState {

}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct CommandQueue {}

impl core::CommandQueue<Resources, CommandBuffer> for CommandQueue {
    fn make_command_buffer(&self) -> core::Result<CommandBuffer> { unimplemented!() }

    fn wait_idle(&self) { unimplemented!() }

    fn submit_commands(&self,
                       buffers: &[&CommandBuffer],
                       fence: Option<&Fence>)
                       -> core::Result<()> { unimplemented!() }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Factory {}

impl core::Factory<Resources> for Factory {
    fn make_fence(&self, descriptor: &core::FenceDescription) -> core::Result<Fence> { unimplemented!() }
    fn make_semaphore(&self, descriptor: &core::SemaphoreDescription) -> core::Result<Semaphore> { unimplemented!() }
    fn make_render_pass(&self, description: &core::RenderPassDescription) -> core::Result<RenderPass> { unimplemented!() }
    fn make_framebuffer(&self,
                        description: &core::FramebufferDescription<RenderPass, ImageView>)
                        -> core::Result<Framebuffer> { unimplemented!() }

    fn make_heap(&self, description: &core::HeapDescription) -> core::Result<Heap> { unimplemented!() }

    /// Creates a buffer view.
    fn make_buffer_view(&self,
                        description: &core::BufferViewDescription<Buffer>)
                        -> core::Result<BufferView> { unimplemented!() }
    fn make_image_view(&self,
                       description: &core::ImageViewDescription<Image>)
                       -> core::Result<ImageView> { unimplemented!() }
    fn get_buffer_memory_requirements(&self,
                                      description: &core::BufferDescription)
                                      -> core::MemoryRequirements { unimplemented!() }
    fn get_image_memory_requirements(&self, description: &core::ImageDescription) -> core::MemoryRequirements { unimplemented!() }

    fn make_sampler(&self, description: &core::SamplerDescription) -> core::Result<Sampler> { unimplemented!() }

    fn make_shader_module(&self, description: &core::ShaderModuleDescription) -> core::Result<ShaderModule> { unimplemented!() }

    fn make_compute_pipeline(&self,
                             description: &core::ComputePipelineDescription<PipelineLayout,
                                                                      ShaderModule>)
                             -> core::Result<ComputePipeline> { unimplemented!() }

    fn make_graphics_pipeline(&self,
                              description: &core::GraphicsPipelineDescription<RenderPass,
                                                                        PipelineLayout,
                                                                        ShaderModule>)
                              -> core::Result<GraphicsPipeline> { unimplemented!() }

    fn make_stencil_state(&self, description: &core::StencilStateDescription) -> core::Result<StencilState> { unimplemented!() }

    fn make_descriptor_set_layout(&self,
                                  description: &core::DescriptorSetLayoutDescription<Sampler>)
                                  -> core::Result<DescriptorSetLayout> { unimplemented!() }
    fn make_pipeline_layout(&self,
                            description: &core::PipelineLayoutDescription<DescriptorSetLayout>)
                            -> core::Result<PipelineLayout> { unimplemented!() }

    fn make_descriptor_pool(&self,
                            description: &core::DescriptorPoolDescription)
                            -> core::Result<DescriptorPool> { unimplemented!() }
    fn make_descriptor_sets(&self,
                            description: &core::DescriptorSetDescription<DescriptorSetLayout>,
                            pool: &DescriptorPool)
                            -> core::Result<DescriptorSet> { unimplemented!() }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct DeviceCapabilities {}

impl core::DeviceCapabilities for DeviceCapabilities {
    fn limits(&self) -> &core::DeviceLimits {
        unimplemented!()
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Device {
    main_queue: CommandQueue,
    factory: Factory,
    cap: DeviceCapabilities,
}

impl core::Device for Device {
    type Resources = Resources;
    type CommandBuffer = CommandBuffer;
    type CommandQueue = CommandQueue;
    type Factory = Factory;
    type DeviceCapabilities = DeviceCapabilities;
    fn main_queue(&self) -> &Self::CommandQueue { &self.main_queue }
    fn factory(&self) -> &Self::Factory { &self.factory }
    fn capabilities(&self) -> &Self::DeviceCapabilities { &self.cap }
}

#[test]
fn test() {
    Device {
        main_queue: CommandQueue{},
        factory: Factory{},
        cap: DeviceCapabilities{},
    };
}

