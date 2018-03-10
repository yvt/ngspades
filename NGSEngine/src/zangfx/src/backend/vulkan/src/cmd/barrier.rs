//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Barrier` for Vulkan.
use std::sync::Arc;
use std::ops::Range;
use ash::vk;

use base;
use common::Result;

use utils::{translate_access_type_flags, translate_pipeline_stage_flags};
use buffer::Buffer;

/// Implementation of `BarrierBuilder` for Vulkan.
#[derive(Debug)]
pub struct BarrierBuilder {
    data: BarrierData,
}

zangfx_impl_object! { BarrierBuilder: base::BarrierBuilder, ::Debug }

impl BarrierBuilder {
    pub(crate) fn new() -> Self {
        Self {
            data: BarrierData {
                src_stage_mask: vk::PipelineStageFlags::empty(),
                dst_stage_mask: vk::PipelineStageFlags::empty(),
                global_barriers: Vec::new(),
                buffer_barriers: Vec::new(),
                image_barriers: Vec::new(),
            },
        }
    }
}

impl base::BarrierBuilder for BarrierBuilder {
    fn global(
        &mut self,
        src_access: base::AccessTypeFlags,
        dst_access: base::AccessTypeFlags,
    ) -> &mut base::BarrierBuilder {
        self.data.global_barriers.push(vk::MemoryBarrier {
            s_type: vk::StructureType::MemoryBarrier,
            p_next: ::null(),
            src_access_mask: translate_access_type_flags(src_access),
            dst_access_mask: translate_access_type_flags(dst_access),
        });
        self.data.src_stage_mask |=
            translate_pipeline_stage_flags(base::AccessType::union_supported_stages(src_access));
        self.data.dst_stage_mask |=
            translate_pipeline_stage_flags(base::AccessType::union_supported_stages(dst_access));
        self
    }

    fn buffer(
        &mut self,
        src_access: base::AccessTypeFlags,
        dst_access: base::AccessTypeFlags,
        buffer: &base::Buffer,
        range: Option<Range<base::DeviceSize>>,
    ) -> &mut base::BarrierBuilder {
        let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        let range = range.as_ref();
        self.data.buffer_barriers.push(vk::BufferMemoryBarrier {
            s_type: vk::StructureType::BufferMemoryBarrier,
            p_next: ::null(),
            src_access_mask: translate_access_type_flags(src_access),
            dst_access_mask: translate_access_type_flags(dst_access),
            src_queue_family_index: vk::VK_QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::VK_QUEUE_FAMILY_IGNORED,
            buffer: my_buffer.vk_buffer(),
            offset: range.map(|r| r.start).unwrap_or(vk::VK_WHOLE_SIZE),
            size: range.map(|r| r.end - r.start).unwrap_or(vk::VK_WHOLE_SIZE),
        });
        self.data.src_stage_mask |=
            translate_pipeline_stage_flags(base::AccessType::union_supported_stages(src_access));
        self.data.dst_stage_mask |=
            translate_pipeline_stage_flags(base::AccessType::union_supported_stages(dst_access));
        self
    }

    fn image(
        &mut self,
        _src_access: base::AccessTypeFlags,
        _dst_access: base::AccessTypeFlags,
        _image: &base::Image,
        _src_layout: base::ImageLayout,
        _dst_layout: base::ImageLayout,
        _range: &base::ImageSubRange,
    ) -> &mut base::BarrierBuilder {
        unimplemented!()
    }

    fn build(&mut self) -> Result<base::Barrier> {
        Ok(Barrier {
            data: Arc::new(self.data.clone()),
        }.into())
    }
}

/// Implementation of `Barrier` for Vulkan.
#[derive(Debug, Clone)]
pub struct Barrier {
    data: Arc<BarrierData>,
}

zangfx_impl_handle! { Barrier, base::Barrier }

#[derive(Debug, Clone)]
pub(super) struct BarrierData {
    pub src_stage_mask: vk::PipelineStageFlags,
    pub dst_stage_mask: vk::PipelineStageFlags,
    pub global_barriers: Vec<vk::MemoryBarrier>,
    pub buffer_barriers: Vec<vk::BufferMemoryBarrier>,
    pub image_barriers: Vec<vk::ImageMemoryBarrier>,
}

unsafe impl Sync for BarrierData {}
unsafe impl Send for BarrierData {}

impl Barrier {
    pub(super) fn data(&self) -> &BarrierData {
        &*self.data
    }
}
