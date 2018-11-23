//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use ngsenumflags::flags;
use std::ops;

use zangfx_base as base;
use zangfx_base::{Error, ErrorKind};

/// Translates a subset of `vk::Result` values into `core::GenericError`.
///
/// The following input values are permitted:
///
///  - `ErrorOutOfDeviceMemory`
///  - `ErrorDeviceLost`
///
/// `ErrorOutOfHostMemory` is escalated to a panic. (Maybe we should call `alloc::oom::oom()`?)
///
/// Unsupported values are returned unmodified.
pub fn translate_generic_error(result: vk::Result) -> ::std::result::Result<Error, vk::Result> {
    match result {
        vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => Ok(Error::new(ErrorKind::OutOfDeviceMemory)),
        vk::Result::ERROR_DEVICE_LOST => Ok(Error::new(ErrorKind::DeviceLost)),
        vk::Result::ERROR_OUT_OF_HOST_MEMORY => panic!("out of memory"),
        result => Err(result),
    }
}

/// Equivalent to `translate_generic_error(result).unwrap()`.
///
/// That is, following errors are handled with this function:
///
///  - `ErrorOutOfDeviceMemory`
///  - `ErrorDeviceLost`
///  - `ErrorOutOfHostMemory` (escalated to a panic)
///
crate fn translate_generic_error_unwrap(result: vk::Result) -> Error {
    translate_generic_error(result).unwrap()
}

pub(crate) fn translate_map_memory_error(
    result: vk::Result,
) -> ::std::result::Result<Error, vk::Result> {
    match result {
        vk::Result::ERROR_MEMORY_MAP_FAILED => panic!("out of virtual memory space"),
        result => translate_generic_error(result),
    }
}

pub(crate) fn translate_map_memory_error_unwrap(result: vk::Result) -> Error {
    translate_map_memory_error(result).unwrap()
}

crate fn translate_memory_req(req: &vk::MemoryRequirements) -> base::MemoryReq {
    base::MemoryReq {
        size: req.size,
        align: req.alignment,
        memory_types: req.memory_type_bits,
    }
}

crate fn translate_shader_stage(value: base::ShaderStage) -> vk::ShaderStageFlags {
    match value {
        base::ShaderStage::Vertex => vk::ShaderStageFlags::VERTEX,
        base::ShaderStage::Fragment => vk::ShaderStageFlags::FRAGMENT,
        base::ShaderStage::Compute => vk::ShaderStageFlags::COMPUTE,
    }
}

crate fn translate_shader_stage_flags(value: base::ShaderStageFlags) -> vk::ShaderStageFlags {
    let mut ret = vk::ShaderStageFlags::empty();
    if value.contains(base::ShaderStage::Vertex) {
        ret |= vk::ShaderStageFlags::VERTEX;
    }
    if value.contains(base::ShaderStage::Fragment) {
        ret |= vk::ShaderStageFlags::FRAGMENT;
    }
    if value.contains(base::ShaderStage::Compute) {
        ret |= vk::ShaderStageFlags::COMPUTE;
    }
    ret
}

crate fn translate_access_type_flags(value: base::AccessTypeFlags) -> vk::AccessFlags {
    let mut ret = vk::AccessFlags::empty();
    if value.contains(base::AccessType::IndirectDrawRead) {
        ret |= vk::AccessFlags::INDIRECT_COMMAND_READ;
    }
    if value.contains(base::AccessType::IndexRead) {
        ret |= vk::AccessFlags::INDEX_READ;
    }
    if value.contains(base::AccessType::VertexAttrRead) {
        ret |= vk::AccessFlags::VERTEX_ATTRIBUTE_READ;
    }
    if value.intersects(
        flags![base::AccessType::{VertexUniformRead | FragmentUniformRead | ComputeUniformRead}],
    ) {
        ret |= vk::AccessFlags::UNIFORM_READ;
    }
    if value.intersects(flags![base::AccessType::{VertexRead | FragmentRead | ComputeRead}]) {
        ret |= vk::AccessFlags::SHADER_READ;
    }
    if value.intersects(flags![base::AccessType::{VertexWrite | FragmentWrite | ComputeWrite}]) {
        ret |= vk::AccessFlags::SHADER_READ;
        ret |= vk::AccessFlags::SHADER_WRITE;
    }
    if value.contains(base::AccessType::ColorRead) {
        ret |= vk::AccessFlags::COLOR_ATTACHMENT_READ;
    }
    if value.contains(base::AccessType::ColorWrite) {
        ret |= vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
    }
    if value.contains(base::AccessType::DsRead) {
        ret |= vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ;
    }
    if value.contains(base::AccessType::DsWrite) {
        ret |= vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
    }
    if value.contains(base::AccessType::CopyRead) {
        ret |= vk::AccessFlags::TRANSFER_READ;
    }
    if value.contains(base::AccessType::CopyWrite) {
        ret |= vk::AccessFlags::TRANSFER_WRITE;
    }
    ret
}

crate fn translate_pipeline_stage_flags(value: base::StageFlags) -> vk::PipelineStageFlags {
    let mut ret = vk::PipelineStageFlags::empty();
    if value.contains(base::Stage::IndirectDraw) {
        ret |= vk::PipelineStageFlags::DRAW_INDIRECT;
    }
    if value.contains(base::Stage::VertexInput) {
        ret |= vk::PipelineStageFlags::VERTEX_INPUT;
    }
    if value.contains(base::Stage::Vertex) {
        ret |= vk::PipelineStageFlags::VERTEX_SHADER;
    }
    if value.contains(base::Stage::Fragment) {
        ret |= vk::PipelineStageFlags::FRAGMENT_SHADER;
    }
    if value.contains(base::Stage::EarlyFragTests) {
        ret |= vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS;
    }
    if value.contains(base::Stage::LateFragTests) {
        ret |= vk::PipelineStageFlags::LATE_FRAGMENT_TESTS;
    }
    if value.contains(base::Stage::RenderOutput) {
        ret |= vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
    }
    if value.contains(base::Stage::Compute) {
        ret |= vk::PipelineStageFlags::COMPUTE_SHADER;
    }
    if value.contains(base::Stage::Copy) {
        ret |= vk::PipelineStageFlags::TRANSFER;
    }
    ret
}

crate fn translate_image_subresource_range(
    value: &base::ImageSubRange,
    aspect_mask: vk::ImageAspectFlags,
) -> vk::ImageSubresourceRange {
    let mip_levels = value.mip_levels.as_ref();
    let layers = value.layers.as_ref();
    vk::ImageSubresourceRange {
        aspect_mask,
        base_mip_level: mip_levels.map(|x| x.start).unwrap_or(0),
        base_array_layer: layers.map(|x| x.start).unwrap_or(0),
        level_count: mip_levels
            .map(|x| x.end - x.start)
            .unwrap_or(vk::REMAINING_MIP_LEVELS),
        layer_count: layers
            .map(|x| x.end - x.start)
            .unwrap_or(vk::REMAINING_ARRAY_LAYERS),
    }
}

crate fn translate_image_subresource_layers(
    value: &base::ImageLayerRange,
    aspect_mask: vk::ImageAspectFlags,
) -> vk::ImageSubresourceLayers {
    let ref layers = value.layers;
    vk::ImageSubresourceLayers {
        aspect_mask,
        mip_level: value.mip_level,
        base_array_layer: layers.start,
        layer_count: layers.end - layers.start,
    }
}

crate fn translate_image_aspect(value: base::ImageAspect) -> vk::ImageAspectFlags {
    match value {
        base::ImageAspect::Color => vk::ImageAspectFlags::COLOR,
        base::ImageAspect::Depth => vk::ImageAspectFlags::DEPTH,
        base::ImageAspect::Stencil => vk::ImageAspectFlags::STENCIL,
    }
}

crate fn translate_compare_op(value: base::CmpFn) -> vk::CompareOp {
    match value {
        base::CmpFn::Never => vk::CompareOp::NEVER,
        base::CmpFn::Less => vk::CompareOp::LESS,
        base::CmpFn::Equal => vk::CompareOp::EQUAL,
        base::CmpFn::LessEqual => vk::CompareOp::LESS_OR_EQUAL,
        base::CmpFn::Greater => vk::CompareOp::GREATER,
        base::CmpFn::NotEqual => vk::CompareOp::NOT_EQUAL,
        base::CmpFn::GreaterEqual => vk::CompareOp::GREATER_OR_EQUAL,
        base::CmpFn::Always => vk::CompareOp::ALWAYS,
    }
}

crate fn translate_rect2d_u32(value: &base::Rect2D<u32>) -> vk::Rect2D {
    vk::Rect2D {
        offset: vk::Offset2D {
            x: value.min[0] as i32,
            y: value.min[1] as i32,
        },
        extent: vk::Extent2D {
            width: value.max[0].saturating_sub(value.min[0]),
            height: value.max[1].saturating_sub(value.min[1]),
        },
    }
}

crate fn clip_rect2d_u31(value: vk::Rect2D) -> vk::Rect2D {
    use std::cmp::min;
    vk::Rect2D {
        offset: value.offset,
        extent: vk::Extent2D {
            width: min(value.extent.width, 0x7fffffffu32 - value.offset.x as u32),
            height: min(value.extent.height, 0x7fffffffu32 - value.offset.y as u32),
        },
    }
}

crate fn translate_bool(value: bool) -> vk::Bool32 {
    if value {
        vk::TRUE
    } else {
        vk::FALSE
    }
}

crate fn translate_sample_count(value: u32) -> vk::SampleCountFlags {
    vk::SampleCountFlags::from_raw(value)
}

crate fn translate_color_channel_flags(value: base::ColorChannelFlags) -> vk::ColorComponentFlags {
    let mut mask = vk::ColorComponentFlags::empty();

    if value.contains(base::ColorChannel::Red) {
        mask |= vk::ColorComponentFlags::R;
    }
    if value.contains(base::ColorChannel::Green) {
        mask |= vk::ColorComponentFlags::G;
    }
    if value.contains(base::ColorChannel::Blue) {
        mask |= vk::ColorComponentFlags::B;
    }
    if value.contains(base::ColorChannel::Alpha) {
        mask |= vk::ColorComponentFlags::A;
    }

    mask
}

crate fn offset_range<T: ops::Add<RHS>, RHS: Clone>(
    range: ops::Range<T>,
    offset: RHS,
) -> ops::Range<T::Output> {
    range.start + offset.clone()..range.end + offset
}

use crate::device::DeviceRef;
use crate::resstate::QueueId;

/// Implements the `queue` property of builders.
#[derive(Debug, Default, Clone, Copy)]
crate struct QueueIdBuilder(Option<QueueId>);

impl QueueIdBuilder {
    crate fn new() -> Self {
        Default::default()
    }

    crate fn set(&mut self, queue: &base::CmdQueueRef) {
        self.0 = Some(queue_id_from_queue(queue));
    }

    crate fn get(&self, device: &DeviceRef) -> QueueId {
        self.0.unwrap_or_else(|| device.default_resstate_queue())
    }
}

crate fn queue_id_from_queue(queue: &base::CmdQueueRef) -> QueueId {
    use crate::cmd::queue::CmdQueue;
    let my_cmd_queue: &CmdQueue = queue.query_ref().expect("bad cmd queue type");
    my_cmd_queue.resstate_queue_id()
}
