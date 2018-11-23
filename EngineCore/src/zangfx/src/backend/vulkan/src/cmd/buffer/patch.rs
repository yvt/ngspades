//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implements command buffer patching which is done during command buffer
//! submission.
use ash::version::*;
use ash::{prelude::VkResult, vk};
use ngsenumflags::flags;
use smallvec::SmallVec;

use zangfx_base as base;

use crate::device::DeviceRef;
use crate::image::ImageStateAddresser;
use crate::limits::DeviceTrait;
use crate::resstate::Queue;
use crate::utils::{
    translate_access_type_flags, translate_image_subresource_range, translate_pipeline_stage_flags,
};

use super::CmdBufferData;

/// Return the Vulkan command buffer stored in `cell`. A new one will be created
/// and stored to `cell` if there's none.
unsafe fn ensure_cmd_buffer(
    cell: &mut Option<vk::CommandBuffer>,
    device: &DeviceRef,
    vk_cmd_pool: vk::CommandPool,
) -> VkResult<vk::CommandBuffer> {
    if cell.is_none() {
        let vk_device = device.vk_device();

        let vk_cmd_buffer = vk_device
            .allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
                p_next: crate::null(),
                command_pool: vk_cmd_pool,
                level: vk::CommandBufferLevel::PRIMARY,
                command_buffer_count: 1,
            }).map(|cbs| cbs[0])?;

        *cell = Some(vk_cmd_buffer);

        vk_device.begin_command_buffer(
            vk_cmd_buffer,
            &vk::CommandBufferBeginInfo {
                s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
                p_next: crate::null(),
                flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                p_inheritance_info: crate::null(),
            },
        )?;
    }

    Ok(cell.unwrap())
}

impl CmdBufferData {
    /// Resolve unresolved barriers in a `CmdBufferData` and modifies
    /// `VkCommandBuffer`s. During this, tracked resource states may be updated
    /// using a supplied `resstate_queue`.
    ///
    /// Furthermore, it finalizes all Vulkan command buffers.
    ///
    /// This will be called in the sequential order in which command buffers are
    /// submitted.
    crate fn finalize(&mut self, resstate_queue: &mut Queue) -> VkResult<()> {
        use crate::resstate::Resource; // for `tracked_state()`

        let ref device = self.device;
        let vk_device = device.vk_device();

        let traits = device.caps().info.traits;

        let vk_cmd_pool = self.vk_cmd_pool;

        // The Vulkan command buffer of the previous pass.
        let mut vk_prev_cmd_buffer = None;
        let ref mut vk_prelude_cmd_buffer = self.vk_prelude_cmd_buffer;
        macro_rules! vk_prev_cmd_buffer {
            () => {
                if let Some(vk_cmd_buffer) = vk_prev_cmd_buffer {
                    vk_cmd_buffer
                } else {
                    ensure_cmd_buffer(vk_prelude_cmd_buffer, device, vk_cmd_pool)?;
                    vk_prev_cmd_buffer = Some(vk_prelude_cmd_buffer.unwrap());
                    vk_prev_cmd_buffer.unwrap()
                }
            };
        }

        let ref mut vk_image_barriers = self.temp.vk_image_barriers;

        let ref ref_table = self.ref_table;

        let max_num_wait_fences = (self.passes.iter())
            .map(|pass| pass.wait_fences.len())
            .max()
            .unwrap_or(0);
        let mut vk_events = SmallVec::<[vk::Event; 16]>::with_capacity(max_num_wait_fences);

        // Iterate through passes in the execution order...
        for pass in self.passes.iter() {
            let mut event_src_access = base::AccessTypeFlags::empty();
            let mut event_src_stages = vk::PipelineStageFlags::empty();
            let mut barrier_dst_access = base::AccessTypeFlags::empty();

            vk_events.clear();
            for &(fence_i, dst_access) in pass.wait_fences.iter() {
                let fence = ref_table.fences.get_by_index(fence_i).resource;
                vk_events.push(fence.vk_event());

                let sched_data = fence.tracked_state().latest_mut(resstate_queue);
                let src_access = sched_data
                    .src_access
                    .expect("attempted to wait on an unsignalled fence");
                event_src_access |= src_access;

                let src_stages = base::AccessType::union_supported_stages(src_access);
                event_src_stages |= if src_stages.is_empty() {
                    vk::PipelineStageFlags::TOP_OF_PIPE
                } else {
                    translate_pipeline_stage_flags(src_stages)
                };

                barrier_dst_access |= dst_access;
            }

            for image_barrier in pass.image_barriers.iter() {
                barrier_dst_access |= image_barrier.access;
            }

            vk_image_barriers.clear();
            vk_image_barriers.reserve(pass.image_barriers.len());
            for image_barrier in pass.image_barriers.iter() {
                let image_i = image_barrier.image_index;
                let unit_i = image_barrier.unit_index;
                let initial_layout = image_barrier.initial_layout;
                let final_layout = image_barrier.final_layout;

                let image = ref_table.images.get_by_index(image_i).resource;
                let sched_data = image.tracked_state().latest_mut(resstate_queue);

                let old_layout = sched_data.units[unit_i].layout;

                if old_layout != Some(initial_layout)
                    && initial_layout != vk::ImageLayout::UNDEFINED
                {
                    let addresser = ImageStateAddresser::from_image(image);

                    vk_image_barriers.push(vk::ImageMemoryBarrier {
                        s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                        p_next: crate::null(),
                        src_access_mask: translate_access_type_flags(event_src_access),
                        dst_access_mask: translate_access_type_flags(barrier_dst_access),
                        old_layout: if let Some(layout) = old_layout {
                            layout
                        } else {
                            vk::ImageLayout::UNDEFINED
                        },
                        new_layout: initial_layout,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        image: image.vk_image(),
                        subresource_range: translate_image_subresource_range(
                            &addresser.subrange_for_index(unit_i).into(),
                            image.aspects(),
                        ),
                    });
                }

                sched_data.units[unit_i].layout = Some(final_layout);
            }

            // Events are not supported by MoltenVK and will cause
            // a `FeatureNotPresent` error
            if vk_events.len() > 0 && !traits.intersects(DeviceTrait::MoltenVK) {
                let src_stage = event_src_stages;
                let dst_stage = base::AccessType::union_supported_stages(barrier_dst_access);

                // This might be too conservative especially if there are image
                // layout transitions with overlapping access type flags.
                // However, given limited information, this is best we can do
                // here.
                let barrier_src_access = event_src_access;
                let barrier = vk::MemoryBarrier {
                    s_type: vk::StructureType::MEMORY_BARRIER,
                    p_next: crate::null(),
                    src_access_mask: translate_access_type_flags(
                        // Read-to-write hazards need only pipeline barrier to deal with
                        barrier_src_access
                            & flags![base::AccessType::{VertexWrite | FragmentWrite |
                            ColorWrite | DsWrite | CopyWrite | ComputeWrite}],
                    ),
                    dst_access_mask: translate_access_type_flags(barrier_dst_access),
                };

                unsafe {
                    vk_device.fp_v1_0().cmd_wait_events(
                        vk_prev_cmd_buffer!(),
                        vk_events.len() as u32,
                        vk_events.as_ptr(),
                        src_stage,
                        if dst_stage.is_empty() {
                            vk::PipelineStageFlags::BOTTOM_OF_PIPE
                        } else {
                            translate_pipeline_stage_flags(dst_stage)
                        },
                        if barrier.src_access_mask.is_empty() {
                            0
                        } else {
                            1
                        },
                        &barrier,
                        0,
                        crate::null(),
                        vk_image_barriers.len() as u32,
                        vk_image_barriers.as_ptr(),
                    );
                }
            } else if vk_image_barriers.len() > 0 {
                let dst_stage = base::AccessType::union_supported_stages(barrier_dst_access);

                unsafe {
                    vk_device.cmd_pipeline_barrier(
                        vk_prev_cmd_buffer!(),
                        vk::PipelineStageFlags::TOP_OF_PIPE,
                        if dst_stage.is_empty() {
                            vk::PipelineStageFlags::BOTTOM_OF_PIPE
                        } else {
                            translate_pipeline_stage_flags(dst_stage)
                        },
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &vk_image_barriers,
                    );
                }
            }

            // Update `src_access` of signaled fences
            for &(fence_i, src_access) in pass.signal_fences.iter() {
                let fence = ref_table.fences.get_by_index(fence_i).resource;
                let sched_data = fence.tracked_state().latest_mut(resstate_queue);
                sched_data.src_access = Some(src_access);
            }

            for &(image_i, unit_i, layout) in pass.image_layout_overrides.iter() {
                let image = ref_table.images.get_by_index(image_i).resource;
                let sched_data = image.tracked_state().latest_mut(resstate_queue);

                sched_data.units[unit_i].layout = Some(layout);
            }

            vk_prev_cmd_buffer = Some(pass.vk_cmd_buffer);
        }

        // End all command buffers
        unsafe {
            if let Some(vk_cmd_buffer) = vk_prelude_cmd_buffer {
                vk_device.end_command_buffer(*vk_cmd_buffer)?;
            }

            for pass in self.passes.iter() {
                vk_device.end_command_buffer(pass.vk_cmd_buffer)?;
            }
        }

        Ok(())
    }
}
