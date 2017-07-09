//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::vk;
use ash::version::DeviceV1_0;
use std::{ptr, ffi};

use {RefEqArc, DeviceRef, AshDevice, translate_generic_error_unwrap};
use imp::{self, ShaderModule, PipelineLayout};

pub struct GraphicsPipeline<T: DeviceRef> {
    data: RefEqArc<GraphicsPipelineData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for GraphicsPipeline<T> => data
}

#[derive(Debug)]
struct GraphicsPipelineData<T: DeviceRef> {
    device_ref: T,
    handle: vk::Pipeline,
}

impl<T: DeviceRef> core::GraphicsPipeline for GraphicsPipeline<T> {}

impl<T: DeviceRef> Drop for GraphicsPipelineData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.destroy_pipeline(self.handle, self.device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> GraphicsPipeline<T> {
    pub(crate) fn new(
        device_ref: &T,
        desc: &imp::GraphicsPipelineDescription<T>,
    ) -> core::Result<Self> {
        unimplemented!()
    }
}

pub struct StencilState<T: DeviceRef> {
    data: RefEqArc<StencilStateData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for StencilState<T> => data
}

#[derive(Debug)]
struct StencilStateData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::StencilState for StencilState<T> {}


pub struct ComputePipeline<T: DeviceRef> {
    data: RefEqArc<ComputePipelineData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for ComputePipeline<T> => data
}

#[derive(Debug)]
struct ComputePipelineData<T: DeviceRef> {
    device_ref: T,
    handle: vk::Pipeline,
}

impl<T: DeviceRef> core::ComputePipeline for ComputePipeline<T> {}

impl<T: DeviceRef> Drop for ComputePipelineData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.destroy_pipeline(self.handle, self.device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> ComputePipeline<T> {
    pub(crate) fn new(
        device_ref: &T,
        desc: &imp::ComputePipelineDescription<T>,
    ) -> core::Result<Self> {
        let stage = translate_shader_stage_description(&desc.shader_stage);
        let info = vk::ComputePipelineCreateInfo {
            s_type: vk::StructureType::ComputePipelineCreateInfo,
            p_next: ptr::null(),
            flags: vk::PipelineCreateFlags::empty(),
            stage: stage.0,
            layout: desc.pipeline_layout.handle(),
            base_pipeline_handle: vk::Pipeline::null(),
            base_pipeline_index: -1,
        };

        let device_ref = device_ref.clone();
        let cache = vk::PipelineCache::null();
        let handle;
        {
            let device: &AshDevice = device_ref.device();
            handle = unsafe {
                device.create_compute_pipelines(cache, &[info], device_ref.allocation_callbacks())
            }.map_err(|e| translate_pipeline_creation_error_unwrap(&device_ref, e))?
                [0];
        }

        Ok(ComputePipeline {
            data: RefEqArc::new(ComputePipelineData { device_ref, handle }),
        })
    }

    pub(crate) fn device_ref(&self) -> &T {
        &self.data.device_ref
    }

    pub fn handle(&self) -> vk::Pipeline {
        self.data.handle
    }
}

fn translate_pipeline_creation_error_unwrap<T: DeviceRef>(
    device_ref: &T,
    (pipelines, error): (Vec<vk::Pipeline>, vk::Result),
) -> core::GenericError {
    let device: &AshDevice = device_ref.device();

    // First, destroy all successfully created pipelines
    for pl in pipelines {
        if pl != vk::Pipeline::null() {
            unsafe { device.destroy_pipeline(pl, device_ref.allocation_callbacks()) };
        }
    }

    // And then convert the error code
    translate_generic_error_unwrap(error)
}

/// Constructs `vk::PipelineShaderStageCreateInfo` from `core::ShaderStageDescription`.
///
/// Returns a created `vk::PipelineShaderStageCreateInfo` and `CString`.
/// The returned `CString` should live at least as long as the `vk::PipelineShaderStageCreateInfo` is used.
fn translate_shader_stage_description<T: DeviceRef>(
    desc: &core::ShaderStageDescription<ShaderModule<T>>,
) -> (vk::PipelineShaderStageCreateInfo, ffi::CString) {
    let stage = match desc.stage {
        core::ShaderStage::Vertex => vk::SHADER_STAGE_VERTEX_BIT,
        core::ShaderStage::Fragment => vk::SHADER_STAGE_FRAGMENT_BIT,
        core::ShaderStage::Compute => vk::SHADER_STAGE_COMPUTE_BIT,
    };

    let name = ffi::CString::new(desc.entry_point_name).unwrap();

    (
        vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PipelineShaderStageCreateInfo,
            p_next: ptr::null(),
            flags: vk::PipelineShaderStageCreateFlags::empty(), // reserved for future use
            stage,
            module: desc.module.handle(),
            p_name: name.as_ptr(),
            p_specialization_info: ptr::null(),
        },
        name,
    )
}
