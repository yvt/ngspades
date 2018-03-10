//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of pipelines for Vulkan.
use std::sync::Arc;
use std::ffi;
use ash::vk;
use ash::version::*;

use base;
use common::{Error, ErrorKind, Result};
use device::DeviceRef;
use shader::Library;
use arg::layout::RootSig;

use utils::{translate_generic_error_unwrap, translate_shader_stage};

/// Constructs `vk::PipelineShaderStageCreateInfo`.
///
/// Returns a created `vk::PipelineShaderStageCreateInfo` and `CString`.
/// The returned `CString` should live at least as long as the
/// `vk::PipelineShaderStageCreateInfo` is used.
fn new_shader_stage_description(
    stage: base::ShaderStage,
    library: &Library,
    entry_point_name: &str,
) -> (vk::PipelineShaderStageCreateInfo, ffi::CString) {
    let stage = translate_shader_stage(stage);

    let name = ffi::CString::new(entry_point_name).unwrap();

    (
        vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PipelineShaderStageCreateInfo,
            p_next: ::null(),
            flags: vk::PipelineShaderStageCreateFlags::empty(), // reserved for future use
            stage,
            module: library.vk_shader_module(),
            p_name: name.as_ptr(),
            p_specialization_info: ::null(),
        },
        name,
    )
}

fn translate_pipeline_creation_error_unwrap(
    device: DeviceRef,
    (pipelines, error): (Vec<vk::Pipeline>, vk::Result),
) -> Error {
    let device = device.vk_device();

    // First, destroy all successfully created pipelines
    for pl in pipelines {
        if pl != vk::Pipeline::null() {
            unsafe { device.destroy_pipeline(pl, None) };
        }
    }

    // And then convert the error code
    translate_generic_error_unwrap(error)
}

/// Implementation of `ComputePipelineBuilder` for Vulkan.
#[derive(Debug)]
pub struct ComputePipelineBuilder {
    device: DeviceRef,
    compute_shader: Option<(Library, String)>,
    root_sig: Option<RootSig>,
}

zangfx_impl_object! { ComputePipelineBuilder: base::ComputePipelineBuilder, ::Debug }

impl ComputePipelineBuilder {
    pub(super) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            compute_shader: None,
            root_sig: None,
        }
    }
}

impl base::ComputePipelineBuilder for ComputePipelineBuilder {
    fn compute_shader(
        &mut self,
        library: &base::Library,
        entry_point: &str,
    ) -> &mut base::ComputePipelineBuilder {
        let my_library: &Library = library.downcast_ref().expect("bad library type");
        self.compute_shader = Some((my_library.clone(), entry_point.to_owned()));
        self
    }

    fn root_sig(&mut self, v: &base::RootSig) -> &mut base::ComputePipelineBuilder {
        let my_root_sig: &RootSig = v.downcast_ref().expect("bad root signature type");
        self.root_sig = Some(my_root_sig.clone());
        self
    }

    fn build(&mut self) -> Result<base::ComputePipeline> {
        let compute_shader = self.compute_shader
            .as_ref()
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "compute_shader"))?;
        let root_sig = self.root_sig
            .as_ref()
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "root_sig"))?;

        let stage = new_shader_stage_description(
            base::ShaderStage::Compute,
            &compute_shader.0,
            &compute_shader.1,
        );

        let info = vk::ComputePipelineCreateInfo {
            s_type: vk::StructureType::ComputePipelineCreateInfo,
            p_next: ::null(),
            flags: vk::PipelineCreateFlags::empty(),
            stage: stage.0,
            layout: root_sig.vk_pipeline_layout(),
            base_pipeline_handle: vk::Pipeline::null(),
            base_pipeline_index: -1,
        };

        let cache = vk::PipelineCache::null();

        let vk_device = self.device.vk_device();
        let vk_pipeline = unsafe { vk_device.create_compute_pipelines(cache, &[info], None) }
            .map_err(|e| translate_pipeline_creation_error_unwrap(self.device, e))?[0];
        Ok(unsafe { ComputePipeline::from_raw(self.device, vk_pipeline) }.into())
    }
}

/// Implementation of `ComputePipeline` for Vulkan.
#[derive(Debug, Clone)]
pub struct ComputePipeline {
    data: Arc<ComputePipelineData>,
}

zangfx_impl_handle! { ComputePipeline, base::ComputePipeline }

#[derive(Debug)]
struct ComputePipelineData {
    device: DeviceRef,
    vk_pipeline: vk::Pipeline,
}

impl ComputePipeline {
    pub(crate) unsafe fn from_raw(device: DeviceRef, vk_pipeline: vk::Pipeline) -> Self {
        Self {
            data: Arc::new(ComputePipelineData {
                device,
                vk_pipeline,
            }),
        }
    }

    pub fn vk_pipeline(&self) -> vk::Pipeline {
        self.data.vk_pipeline
    }
}

impl Drop for ComputePipelineData {
    fn drop(&mut self) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.destroy_pipeline(self.vk_pipeline, None);
        }
    }
}
