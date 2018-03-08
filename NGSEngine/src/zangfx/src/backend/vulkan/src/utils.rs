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
