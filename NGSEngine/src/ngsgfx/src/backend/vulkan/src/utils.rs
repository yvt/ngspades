//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use core;

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
pub fn translate_generic_error(
    result: vk::Result,
) -> Result<core::GenericError, vk::Result> {
    match result {
        vk::Result::ErrorOutOfDeviceMemory => Ok(core::GenericError::OutOfDeviceMemory),
        vk::Result::ErrorDeviceLost => Ok(core::GenericError::DeviceLost),
        vk::Result::ErrorOutOfHostMemory => panic!("out of memory"),
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
pub fn translate_generic_error_unwrap(result: vk::Result) -> core::GenericError {
    translate_generic_error(result).unwrap()
}

pub(crate) fn translate_map_memory_error(
    result: vk::Result,
) -> Result<core::GenericError, vk::Result> {
    match result {
        vk::Result::ErrorMemoryMapFailed => panic!("out of virtual memory space"),
        result => translate_generic_error(result),
    }
}

pub(crate) fn translate_map_memory_error_unwrap(result: vk::Result) -> core::GenericError {
    translate_map_memory_error(result).unwrap()
}

pub(crate) fn translate_image_layout(value: core::ImageLayout) -> vk::ImageLayout {
    match value {
        core::ImageLayout::Undefined => vk::ImageLayout::Undefined,
        core::ImageLayout::General => vk::ImageLayout::General,
        core::ImageLayout::ColorAttachment => vk::ImageLayout::ColorAttachmentOptimal,
        core::ImageLayout::DepthStencilAttachment => vk::ImageLayout::DepthStencilAttachmentOptimal,
        core::ImageLayout::DepthStencilRead => vk::ImageLayout::DepthStencilReadOnlyOptimal,
        core::ImageLayout::ShaderRead => vk::ImageLayout::ShaderReadOnlyOptimal,
        core::ImageLayout::TransferSource => vk::ImageLayout::TransferSrcOptimal,
        core::ImageLayout::TransferDestination => vk::ImageLayout::TransferDstOptimal,
        core::ImageLayout::Preinitialized => vk::ImageLayout::Preinitialized,
        core::ImageLayout::Present => vk::ImageLayout::PresentSrcKhr,
    }
}

pub(crate) fn translate_image_subresource_range(
    value: &core::ImageSubresourceRange,
    aspect_mask: vk::ImageAspectFlags,
) -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask,
        base_mip_level: value.base_mip_level,
        base_array_layer: value.base_array_layer,
        level_count: value.num_mip_levels.unwrap_or(vk::VK_REMAINING_MIP_LEVELS),
        layer_count: value.num_array_layers.unwrap_or(
            vk::VK_REMAINING_ARRAY_LAYERS,
        ),
    }
}

pub(crate) fn translate_compare_function(value: core::CompareFunction) -> vk::CompareOp {
    match value {
        core::CompareFunction::Never => vk::CompareOp::Never,
        core::CompareFunction::Less => vk::CompareOp::Less,
        core::CompareFunction::Equal => vk::CompareOp::Equal,
        core::CompareFunction::LessEqual => vk::CompareOp::LessOrEqual,
        core::CompareFunction::Greater => vk::CompareOp::Greater,
        core::CompareFunction::NotEqual => vk::CompareOp::NotEqual,
        core::CompareFunction::GreaterEqual => vk::CompareOp::GreaterOrEqual,
        core::CompareFunction::Always => vk::CompareOp::Always,
    }
}

pub(crate) fn translate_access_type_flags(value: core::AccessTypeFlags) -> vk::AccessFlags {
    let mut ret = vk::AccessFlags::empty();
    if value.contains(core::AccessType::IndirectCommandRead) {
        ret |= vk::ACCESS_INDIRECT_COMMAND_READ_BIT;
    }
    if value.contains(core::AccessType::IndexRead) {
        ret |= vk::ACCESS_INDEX_READ_BIT;
    }
    if value.contains(core::AccessType::VertexAttributeRead) {
        ret |= vk::ACCESS_VERTEX_ATTRIBUTE_READ_BIT;
    }
    if value.contains(core::AccessType::UniformRead) {
        ret |= vk::ACCESS_UNIFORM_READ_BIT;
    }
    if value.contains(core::AccessType::InputAttachmentRead) {
        ret |= vk::ACCESS_INPUT_ATTACHMENT_READ_BIT;
    }
    if value.contains(core::AccessType::ShaderRead) {
        ret |= vk::ACCESS_SHADER_READ_BIT;
    }
    if value.contains(core::AccessType::ShaderWrite) {
        ret |= vk::ACCESS_SHADER_WRITE_BIT;
    }
    if value.contains(core::AccessType::ColorAttachmentRead) {
        ret |= vk::ACCESS_COLOR_ATTACHMENT_READ_BIT;
    }
    if value.contains(core::AccessType::ColorAttachmentWrite) {
        ret |= vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT;
    }
    if value.contains(core::AccessType::DepthStencilAttachmentRead) {
        ret |= vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT;
    }
    if value.contains(core::AccessType::DepthStencilAttachmentWrite) {
        ret |= vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT;
    }
    if value.contains(core::AccessType::TransferRead) {
        ret |= vk::ACCESS_TRANSFER_READ_BIT;
    }
    if value.contains(core::AccessType::TransferWrite) {
        ret |= vk::ACCESS_TRANSFER_WRITE_BIT;
    }
    if value.contains(core::AccessType::HostRead) {
        ret |= vk::ACCESS_HOST_READ_BIT;
    }
    if value.contains(core::AccessType::HostWrite) {
        ret |= vk::ACCESS_HOST_WRITE_BIT;
    }
    if value.contains(core::AccessType::MemoryRead) {
        ret |= vk::ACCESS_MEMORY_READ_BIT;
    }
    if value.contains(core::AccessType::MemoryWrite) {
        ret |= vk::ACCESS_MEMORY_WRITE_BIT;
    }
    ret
}

pub(crate) fn translate_pipeline_stage_flags(value: core::PipelineStageFlags) -> vk::PipelineStageFlags {
    let mut ret = vk::PipelineStageFlags::empty();
    if value.contains(core::PipelineStage::TopOfPipe) {
        ret |= vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT;
    }
    if value.contains(core::PipelineStage::DrawIndirect) {
        ret |= vk::PIPELINE_STAGE_DRAW_INDIRECT_BIT;
    }
    if value.contains(core::PipelineStage::VertexInput) {
        ret |= vk::PIPELINE_STAGE_VERTEX_INPUT_BIT;
    }
    if value.contains(core::PipelineStage::VertexShader) {
        ret |= vk::PIPELINE_STAGE_VERTEX_SHADER_BIT;
    }
    if value.contains(core::PipelineStage::FragmentShader) {
        ret |= vk::PIPELINE_STAGE_FRAGMENT_SHADER_BIT;
    }
    if value.contains(core::PipelineStage::EarlyFragmentTests) {
        ret |= vk::PIPELINE_STAGE_EARLY_FRAGMENT_TESTS_BIT;
    }
    if value.contains(core::PipelineStage::LateFragmentTests) {
        ret |= vk::PIPELINE_STAGE_LATE_FRAGMENT_TESTS_BIT;
    }
    if value.contains(core::PipelineStage::ColorAttachmentOutput) {
        ret |= vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
    }
    if value.contains(core::PipelineStage::ComputeShader) {
        ret |= vk::PIPELINE_STAGE_COMPUTE_SHADER_BIT;
    }
    if value.contains(core::PipelineStage::Transfer) {
        ret |= vk::PIPELINE_STAGE_TRANSFER_BIT;
    }
    if value.contains(core::PipelineStage::BottomOfPipe) {
        ret |= vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT;
    }
    if value.contains(core::PipelineStage::Host) {
        ret |= vk::PIPELINE_STAGE_HOST_BIT;
    }
    if value.contains(core::PipelineStage::AllGraphics) {
        ret |= vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT;
    }
    if value.contains(core::PipelineStage::AllCommands) {
        ret |= vk::PIPELINE_STAGE_ALL_COMMANDS_BIT;
    }
    ret
}

pub(crate) fn translate_shader_stage(value: core::ShaderStage) -> vk::ShaderStageFlags {
    match value {
        core::ShaderStage::Vertex => vk::SHADER_STAGE_VERTEX_BIT,
        core::ShaderStage::Fragment => vk::SHADER_STAGE_FRAGMENT_BIT,
        core::ShaderStage::Compute => vk::SHADER_STAGE_COMPUTE_BIT,
    }
}

pub(crate) fn translate_shader_stage_flags(value: core::ShaderStageFlags) -> vk::ShaderStageFlags {
    let mut ret = vk::ShaderStageFlags::empty();
    if value.contains(core::ShaderStage::Vertex) {
        ret |= vk::SHADER_STAGE_VERTEX_BIT;
    }
    if value.contains(core::ShaderStage::Fragment) {
        ret |= vk::SHADER_STAGE_FRAGMENT_BIT;
    }
    if value.contains(core::ShaderStage::Compute) {
        ret |= vk::SHADER_STAGE_COMPUTE_BIT;
    }
    ret
}

pub(crate) fn translate_rect2d_u32(value: &core::Rect2D<u32>) -> vk::Rect2D {
    vk::Rect2D {
        offset: vk::Offset2D {
            x: value.min.x as i32,
            y: value.min.y as i32,
        },
        extent: vk::Extent2D {
            width: value.max.x.saturating_sub(value.min.x),
            height: value.max.y.saturating_sub(value.min.y),
        },
    }
}