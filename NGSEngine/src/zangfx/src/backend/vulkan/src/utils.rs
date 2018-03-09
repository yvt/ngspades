//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use ash::version::*;

use base;
use common::{Error, ErrorKind, Result};

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
        vk::Result::ErrorOutOfDeviceMemory => Ok(Error::new(ErrorKind::OutOfDeviceMemory)),
        vk::Result::ErrorDeviceLost => Ok(Error::new(ErrorKind::DeviceLost)),
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
pub fn translate_generic_error_unwrap(result: vk::Result) -> Error {
    translate_generic_error(result).unwrap()
}

pub(crate) fn translate_map_memory_error(
    result: vk::Result,
) -> ::std::result::Result<Error, vk::Result> {
    match result {
        vk::Result::ErrorMemoryMapFailed => panic!("out of virtual memory space"),
        result => translate_generic_error(result),
    }
}

pub(crate) fn translate_map_memory_error_unwrap(result: vk::Result) -> Error {
    translate_map_memory_error(result).unwrap()
}

pub fn get_memory_req(vk_device: &::AshDevice, obj: base::ResourceRef) -> Result<base::MemoryReq> {
    use buffer;
    let req = match obj {
        base::ResourceRef::Buffer(buffer) => {
            let our_buffer: &buffer::Buffer = buffer.downcast_ref().expect("bad buffer type");
            vk_device.get_buffer_memory_requirements(our_buffer.vk_buffer())
        }
        base::ResourceRef::Image(_image) => unimplemented!(),
    };
    Ok(base::MemoryReq {
        size: req.size,
        align: req.alignment,
        memory_types: req.memory_type_bits,
    })
}

pub fn translate_shader_stage(value: base::ShaderStage) -> vk::ShaderStageFlags {
    match value {
        base::ShaderStage::Vertex => vk::SHADER_STAGE_VERTEX_BIT,
        base::ShaderStage::Fragment => vk::SHADER_STAGE_FRAGMENT_BIT,
        base::ShaderStage::Compute => vk::SHADER_STAGE_COMPUTE_BIT,
    }
}

pub fn translate_shader_stage_flags(value: base::ShaderStageFlags) -> vk::ShaderStageFlags {
    let mut ret = vk::ShaderStageFlags::empty();
    if value.contains(base::ShaderStage::Vertex) {
        ret |= vk::SHADER_STAGE_VERTEX_BIT;
    }
    if value.contains(base::ShaderStage::Fragment) {
        ret |= vk::SHADER_STAGE_FRAGMENT_BIT;
    }
    if value.contains(base::ShaderStage::Compute) {
        ret |= vk::SHADER_STAGE_COMPUTE_BIT;
    }
    ret
}

pub fn translate_access_type_flags(value: base::AccessTypeFlags) -> vk::AccessFlags {
    let mut ret = vk::AccessFlags::empty();
    if value.contains(base::AccessType::IndirectDrawRead) {
        ret |= vk::ACCESS_INDIRECT_COMMAND_READ_BIT;
    }
    if value.contains(base::AccessType::IndexRead) {
        ret |= vk::ACCESS_INDEX_READ_BIT;
    }
    if value.contains(base::AccessType::VertexAttrRead) {
        ret |= vk::ACCESS_VERTEX_ATTRIBUTE_READ_BIT;
    }
    if value.intersects(
        flags![base::AccessType::{VertexUniformRead | FragmentUniformRead | ComputeUniformRead}],
    ) {
        ret |= vk::ACCESS_UNIFORM_READ_BIT;
    }
    if value.intersects(flags![base::AccessType::{VertexRead | FragmentRead | ComputeRead}]) {
        ret |= vk::ACCESS_SHADER_READ_BIT;
    }
    if value.intersects(flags![base::AccessType::{VertexWrite | FragmentWrite | ComputeWrite}]) {
        ret |= vk::ACCESS_SHADER_READ_BIT;
        ret |= vk::ACCESS_SHADER_WRITE_BIT;
    }
    if value.contains(base::AccessType::ColorRead) {
        ret |= vk::ACCESS_COLOR_ATTACHMENT_READ_BIT;
    }
    if value.contains(base::AccessType::ColorWrite) {
        ret |= vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT;
    }
    if value.contains(base::AccessType::DsRead) {
        ret |= vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT;
    }
    if value.contains(base::AccessType::DsWrite) {
        ret |= vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT;
    }
    if value.contains(base::AccessType::CopyRead) {
        ret |= vk::ACCESS_TRANSFER_READ_BIT;
    }
    if value.contains(base::AccessType::CopyWrite) {
        ret |= vk::ACCESS_TRANSFER_WRITE_BIT;
    }
    ret
}

pub fn translate_pipeline_stage_flags(value: base::StageFlags) -> vk::PipelineStageFlags {
    let mut ret = vk::PipelineStageFlags::empty();
    if value.contains(base::Stage::IndirectDraw) {
        ret |= vk::PIPELINE_STAGE_DRAW_INDIRECT_BIT;
    }
    if value.contains(base::Stage::VertexInput) {
        ret |= vk::PIPELINE_STAGE_VERTEX_INPUT_BIT;
    }
    if value.contains(base::Stage::Vertex) {
        ret |= vk::PIPELINE_STAGE_VERTEX_SHADER_BIT;
    }
    if value.contains(base::Stage::Fragment) {
        ret |= vk::PIPELINE_STAGE_FRAGMENT_SHADER_BIT;
    }
    if value.contains(base::Stage::EarlyFragTests) {
        ret |= vk::PIPELINE_STAGE_EARLY_FRAGMENT_TESTS_BIT;
    }
    if value.contains(base::Stage::LateFragTests) {
        ret |= vk::PIPELINE_STAGE_LATE_FRAGMENT_TESTS_BIT;
    }
    if value.contains(base::Stage::RenderOutput) {
        ret |= vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
    }
    if value.contains(base::Stage::Compute) {
        ret |= vk::PIPELINE_STAGE_COMPUTE_SHADER_BIT;
    }
    if value.contains(base::Stage::Copy) {
        ret |= vk::PIPELINE_STAGE_TRANSFER_BIT;
    }
    ret
}
