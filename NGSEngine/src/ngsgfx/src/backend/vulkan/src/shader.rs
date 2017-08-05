//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::vk;
use ash::version::DeviceV1_0;
use std::ptr;

use {RefEqArc, DeviceRef, AshDevice, translate_generic_error_unwrap};

pub struct ShaderModule<T: DeviceRef> {
    data: RefEqArc<ShaderModuleData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for ShaderModule<T> => data
}

#[derive(Debug)]
struct ShaderModuleData<T: DeviceRef> {
    device_ref: T,
    handle: vk::ShaderModule,
}

impl<T: DeviceRef> ShaderModule<T> {
    pub(crate) fn new(device_ref: &T, desc: &core::ShaderModuleDescription) -> core::Result<Self> {
        assert!(
            desc.spirv_code.len() <= (<u32>::max_value() / 4) as usize,
            "shader is too big"
        );
        let info = vk::ShaderModuleCreateInfo {
            s_type: vk::StructureType::ShaderModuleCreateInfo,
            p_next: ptr::null(),
            flags: vk::ShaderModuleCreateFlags::empty(), // reserved for future use
            code_size: desc.spirv_code.len() * 4,
            p_code: desc.spirv_code.as_ptr(),
        };

        let device_ref = device_ref.clone();
        let handle;
        {
            let device: &AshDevice = device_ref.device();
            handle = unsafe {
                device.create_shader_module(&info, device_ref.allocation_callbacks())
            }.map_err(translate_generic_error_unwrap)?;
        }

        Ok(ShaderModule {
            data: RefEqArc::new(ShaderModuleData { device_ref, handle }),
        })
    }

    pub fn handle(&self) -> vk::ShaderModule {
        self.data.handle
    }
}

impl<T: DeviceRef> Drop for ShaderModuleData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe {
            device.destroy_shader_module(self.handle, self.device_ref.allocation_callbacks())
        };
    }
}

impl<T: DeviceRef> core::ShaderModule for ShaderModule<T> {}

impl<T: DeviceRef> core::Marker for ShaderModule<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}
