//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;
use std::ptr;

use imp::{CommandBuffer, SecondaryCommandBuffer, Fence, Image};
use {DeviceRef, Backend, translate_access_type_flags, translate_pipeline_stage_flags,
     translate_image_layout, translate_image_subresource_range, AshDevice};
use super::{CommandPass, SecondaryCommandBufferData};

fn resource_barrier<T: DeviceRef>(
    device: &AshDevice,
    buffer: vk::CommandBuffer,
    source_stage: core::PipelineStageFlags,
    source_access: core::AccessTypeFlags,
    destination_stage: core::PipelineStageFlags,
    destination_access: core::AccessTypeFlags,
    resource: &core::SubresourceWithLayout<Backend<T>>,
) {
    unsafe {
        match resource {
            &core::SubresourceWithLayout::Buffer {
                buffer: buf,
                offset,
                len,
            } => {
                device.cmd_pipeline_barrier(
                    buffer,
                    translate_pipeline_stage_flags(source_stage),
                    translate_pipeline_stage_flags(destination_stage),
                    vk::DependencyFlags::empty(),
                    &[],
                    &[
                        vk::BufferMemoryBarrier {
                            s_type: vk::StructureType::BufferMemoryBarrier,
                            p_next: ptr::null(),
                            src_access_mask: translate_access_type_flags(source_access),
                            dst_access_mask: translate_access_type_flags(destination_access),
                            src_queue_family_index: vk::VK_QUEUE_FAMILY_IGNORED,
                            dst_queue_family_index: vk::VK_QUEUE_FAMILY_IGNORED,
                            buffer: buf.handle(),
                            offset,
                            size: len,
                        },
                    ],
                    &[],
                );
            }
            &core::SubresourceWithLayout::Image {
                image,
                range,
                old_layout,
                new_layout,
            } => {
                device.cmd_pipeline_barrier(
                    buffer,
                    translate_pipeline_stage_flags(source_stage),
                    translate_pipeline_stage_flags(destination_stage),
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[
                        vk::ImageMemoryBarrier {
                            s_type: vk::StructureType::ImageMemoryBarrier,
                            p_next: ptr::null(),
                            src_access_mask: translate_access_type_flags(source_access),
                            dst_access_mask: translate_access_type_flags(destination_access),
                            old_layout: translate_image_layout(old_layout),
                            new_layout: translate_image_layout(new_layout),
                            src_queue_family_index: vk::VK_QUEUE_FAMILY_IGNORED,
                            dst_queue_family_index: vk::VK_QUEUE_FAMILY_IGNORED,
                            image: image.handle(),
                            subresource_range: translate_image_subresource_range(
                                &range,
                                vk::IMAGE_ASPECT_COLOR_BIT, // TODO
                            ),
                        },
                    ],
                );
            }
        }
    }
}

impl<T: DeviceRef> core::BarrierCommandEncoder<Backend<T>> for CommandBuffer<T> {
    fn wait_fence(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        fence: &Fence<T>,
    ) {
        self.expect_action_pass_mut().wait_fences.push((
            fence.clone(),
            stage,
            access,
        ));
    }

    fn update_fence(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        fence: &Fence<T>,
    ) {
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
        let &mut CommandPass { buffer, .. } = self.expect_action_pass_mut();
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
        self.exepct_active_mut().wait_fences.push((
            fence.clone(),
            stage,
            access,
        ));
    }

    fn update_fence(
        &mut self,
        stage: core::PipelineStageFlags,
        access: core::AccessTypeFlags,
        fence: &Fence<T>,
    ) {
        self.exepct_active_mut().update_fences.push((
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
        let &mut SecondaryCommandBufferData {
            ref device_ref,
            ref buffer,
            ..
        } = self.exepct_active_mut();
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
