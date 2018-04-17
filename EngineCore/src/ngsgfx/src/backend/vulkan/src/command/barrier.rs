//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;
use std::ptr;

use imp::{CommandBuffer, SecondaryCommandBuffer, Fence};
use {DeviceRef, Backend, translate_access_type_flags, translate_pipeline_stage_flags,
     translate_image_layout, translate_image_subresource_range, AshDevice};
use super::{CommandPass, SecondaryCommandBufferData};

pub(crate) enum VkResourceBarrier {
    Buffer([vk::BufferMemoryBarrier; 1]),
    Image([vk::ImageMemoryBarrier; 1]),
}

impl VkResourceBarrier {
    pub fn translate<T: DeviceRef>(
        resource: &core::SubresourceWithLayout<Backend<T>>,
        src_access_mask: vk::AccessFlags,
        dst_access_mask: vk::AccessFlags,
        src_queue_family_index: u32,
        dst_queue_family_index: u32,
    ) -> Self {
        match resource {
            &core::SubresourceWithLayout::Buffer {
                buffer: buf,
                offset,
                len,
            } => VkResourceBarrier::Buffer(
                [
                    vk::BufferMemoryBarrier {
                        s_type: vk::StructureType::BufferMemoryBarrier,
                        p_next: ptr::null(),
                        src_access_mask,
                        dst_access_mask,
                        src_queue_family_index,
                        dst_queue_family_index,
                        buffer: buf.handle(),
                        offset,
                        size: len,
                    },
                ],
            ),
            &core::SubresourceWithLayout::Image {
                image,
                range,
                old_layout,
                new_layout,
            } => VkResourceBarrier::Image(
                [
                    vk::ImageMemoryBarrier {
                        s_type: vk::StructureType::ImageMemoryBarrier,
                        p_next: ptr::null(),
                        src_access_mask,
                        dst_access_mask,
                        old_layout: translate_image_layout(old_layout),
                        new_layout: translate_image_layout(new_layout),
                        src_queue_family_index,
                        dst_queue_family_index,
                        image: image.handle(),
                        subresource_range: translate_image_subresource_range(
                            &range,
                            vk::IMAGE_ASPECT_COLOR_BIT, // TODO
                        ),
                    },
                ],
            ),
        }
    }

    pub fn buffer_memory_barriers(&self) -> &[vk::BufferMemoryBarrier] {
        match self {
            &VkResourceBarrier::Buffer(ref array) => array,
            &VkResourceBarrier::Image(_) => &[],
        }
    }

    pub fn image_memory_barriers(&self) -> &[vk::ImageMemoryBarrier] {
        match self {
            &VkResourceBarrier::Buffer(_) => &[],
            &VkResourceBarrier::Image(ref array) => array,
        }
    }
}

fn resource_barrier<T: DeviceRef>(
    device: &AshDevice,
    buffer: vk::CommandBuffer,
    source_stage: core::PipelineStageFlags,
    source_access: core::AccessTypeFlags,
    destination_stage: core::PipelineStageFlags,
    destination_access: core::AccessTypeFlags,
    resource: &core::SubresourceWithLayout<Backend<T>>,
) {
    let barrier = VkResourceBarrier::translate(
        resource,
        translate_access_type_flags(source_access),
        translate_access_type_flags(destination_access),
        vk::VK_QUEUE_FAMILY_IGNORED,
        vk::VK_QUEUE_FAMILY_IGNORED,
    );
    unsafe {
        device.cmd_pipeline_barrier(
            buffer,
            translate_pipeline_stage_flags(source_stage),
            translate_pipeline_stage_flags(destination_stage),
            vk::DependencyFlags::empty(),
            &[],
            barrier.buffer_memory_barriers(),
            barrier.image_memory_barriers(),
        );
    }
}

impl<T: DeviceRef> core::BarrierCommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn wait_fence(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        fence: &Fence<T>,
    ) {
        if self.encoder_error().is_some() {
            return;
        }

        let ap = self.expect_action_pass_mut();

        fence.expect_waitable_by_iq(ap.internal_queue_index);

        ap.wait_fences.push((fence.clone(), stage, access));
    }

    fn update_fence(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        fence: &Fence<T>,
    ) {
        if self.encoder_error().is_some() {
            return;
        }

        self.expect_action_pass_mut().update_fences.push((
            fence.clone(),
            stage,
            access,
        ));
    }

    fn resource_barrier(
        &mut self,
        source_stage: core::PipelineStageFlags,
        source_access: core::AccessTypeFlags,
        destination_stage: core::PipelineStageFlags,
        destination_access: core::AccessTypeFlags,
        resource: &core::SubresourceWithLayout<Backend<T>>,
    ) {
        if self.encoder_error().is_some() {
            return;
        }

        if let Some(table) = self.dependency_table() {
            match resource {
                &core::SubresourceWithLayout::Buffer { buffer, .. } => {
                    table.insert_buffer(buffer);
                }
                &core::SubresourceWithLayout::Image { image, .. } => {
                    table.insert_image(image);
                }
            }
        }

        let &mut CommandPass { buffer, .. } = self.expect_outside_render_pass_mut();
        let device: &AshDevice = self.data.device_ref.device();

        resource_barrier(
            device,
            buffer,
            source_stage,
            source_access,
            destination_stage,
            destination_access,
            resource,
        )
    }
}

impl<T: DeviceRef> core::BarrierCommandEncoder<Backend<T>> for SecondaryCommandBuffer<T> {
    fn wait_fence(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        fence: &Fence<T>,
    ) {
        if let Some(sbd) = self.expect_active_mut() {
            // `expect_waitable_by_iq` is called when this subpass is ended
            sbd.wait_fences.push((fence.clone(), stage, access));
        }
    }

    fn update_fence(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        fence: &Fence<T>,
    ) {
        if let Some(sbd) = self.expect_active_mut() {
            sbd.update_fences.push((fence.clone(), stage, access));
        }
    }

    fn resource_barrier(
        &mut self,
        source_stage: core::PipelineStageFlags,
        source_access: core::AccessTypeFlags,
        destination_stage: core::PipelineStageFlags,
        destination_access: core::AccessTypeFlags,
        resource: &core::SubresourceWithLayout<Backend<T>>,
    ) {
        if let Some(table) = self.dependency_table() {
            match resource {
                &core::SubresourceWithLayout::Buffer { buffer, .. } => {
                    table.insert_buffer(buffer);
                }
                &core::SubresourceWithLayout::Image { image, .. } => {
                    table.insert_image(image);
                }
            }
        }

        let &mut SecondaryCommandBufferData {
            ref device_ref,
            ref buffer,
            ..
        } = match self.expect_active_mut() {
            Some(x) => x,
            None => return,
        };
        let device: &AshDevice = device_ref.device();

        resource_barrier(
            device,
            *buffer,
            source_stage,
            source_access,
            destination_stage,
            destination_access,
            resource,
        )
    }
}
