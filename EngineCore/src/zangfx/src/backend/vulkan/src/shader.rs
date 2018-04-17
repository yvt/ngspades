//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Library` for Vulkan.
use std::sync::Arc;
use ash::vk;
use ash::version::*;

use base;
use common::{Error, ErrorKind, Result};
use device::DeviceRef;

use utils::translate_generic_error_unwrap;

/// Implementation of `LibraryBuilder` for Vulkan.
#[derive(Debug)]
pub struct LibraryBuilder {
    device: DeviceRef,
    spirv_code: Option<Vec<u32>>,
}

zangfx_impl_object! { LibraryBuilder: base::LibraryBuilder, ::Debug }

impl LibraryBuilder {
    pub(super) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            spirv_code: None,
        }
    }
}

impl base::LibraryBuilder for LibraryBuilder {
    fn spirv_code(&mut self, v: &[u32]) -> &mut base::LibraryBuilder {
        self.spirv_code = Some(Vec::from(v));
        self
    }

    fn build(&mut self) -> Result<base::Library> {
        let spirv_code = self.spirv_code
            .clone()
            .ok_or(Error::new(ErrorKind::InvalidUsage))?;

        if spirv_code.len() >= (<u32>::max_value() / 4) as usize {
            return Err(Error::with_detail(
                ErrorKind::NotSupported,
                "shader is too big",
            ));
        }

        let info = vk::ShaderModuleCreateInfo {
            s_type: vk::StructureType::ShaderModuleCreateInfo,
            p_next: ::null(),
            flags: vk::ShaderModuleCreateFlags::empty(), // reserved for future use
            code_size: spirv_code.len() * 4,
            p_code: spirv_code.as_ptr(),
        };

        let vk_device = self.device.vk_device();
        let vk_shader_mod = unsafe { vk_device.create_shader_module(&info, None) }
            .map_err(translate_generic_error_unwrap)?;
        Ok(unsafe { Library::from_raw(self.device, vk_shader_mod) }.into())
    }
}

/// Implementation of `Library` for Vulkan.
#[derive(Debug, Clone)]
pub struct Library {
    data: Arc<LibraryData>,
}

zangfx_impl_handle! { Library, base::Library }

#[derive(Debug)]
struct LibraryData {
    device: DeviceRef,
    vk_shader_mod: vk::ShaderModule,
}

impl Library {
    pub(crate) unsafe fn from_raw(device: DeviceRef, vk_shader_mod: vk::ShaderModule) -> Self {
        Self {
            data: Arc::new(LibraryData {
                device,
                vk_shader_mod,
            }),
        }
    }

    pub fn vk_shader_module(&self) -> vk::ShaderModule {
        self.data.vk_shader_mod
    }
}

impl Drop for LibraryData {
    fn drop(&mut self) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.destroy_shader_module(self.vk_shader_mod, None);
        }
    }
}
