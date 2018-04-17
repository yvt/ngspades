//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use std::{ptr, ffi, ops, marker};

/// (Relatively) memory-safe wrapper for `DeviceCreateInfo`.
#[derive(Debug, Clone)]
pub struct DeviceCreateInfo {
    pub p_next: *const vk::c_void,
    pub flags: vk::DeviceCreateFlags,
    pub enabled_features: vk::PhysicalDeviceFeatures,
    pub queue_create_infos: Vec<DeviceQueueCreateInfo>,
    pub enabled_layer_names: Vec<ffi::CString>,
    pub enabled_extension_names: Vec<ffi::CString>,
}

/// (Relatively) memory-safe wrapper for `DeviecQueueCreateInfo`.
#[derive(Debug, Clone)]
pub struct DeviceQueueCreateInfo {
    pub p_next: *const vk::c_void,
    pub flags: vk::DeviceQueueCreateFlags,
    pub queue_family_index: u32,
    pub queue_priorities: Vec<vk::c_float>,
}

#[derive(Debug)]
pub struct DeviceCreateInfoRaw<'a> {
    info: vk::DeviceCreateInfo,
    queue_create_infos: Vec<vk::DeviceQueueCreateInfo>,
    enabled_layer_names: Vec<*const vk::c_char>,
    enabled_extension_names: Vec<*const vk::c_char>,
    phantom: marker::PhantomData<&'a DeviceCreateInfo>,
}

impl DeviceCreateInfo {
    pub fn as_raw<'a>(&'a self) -> DeviceCreateInfoRaw<'a> {
        let queue_create_infos: Vec<_> = self.queue_create_infos
            .iter()
            .map(|dqci| {
                vk::DeviceQueueCreateInfo {
                    s_type: vk::StructureType::DeviceQueueCreateInfo,
                    p_next: dqci.p_next,
                    flags: dqci.flags,
                    queue_family_index: dqci.queue_family_index,
                    queue_count: dqci.queue_priorities.len() as u32,
                    p_queue_priorities: dqci.queue_priorities.as_ptr(),
                }
            })
            .collect();

        let enabled_layer_names: Vec<_> = self.enabled_layer_names
            .iter()
            .map(|x| x.as_ptr())
            .collect();

        let enabled_extension_names: Vec<_> = self.enabled_extension_names
            .iter()
            .map(|x| x.as_ptr())
            .collect();

        DeviceCreateInfoRaw {
            info: vk::DeviceCreateInfo {
                s_type: vk::StructureType::DeviceCreateInfo,
                p_next: self.p_next,
                flags: self.flags,
                queue_create_info_count: queue_create_infos.len() as u32,
                p_queue_create_infos: queue_create_infos.as_ptr(),
                enabled_layer_count: enabled_layer_names.len() as u32,
                pp_enabled_layer_names: enabled_layer_names.as_ptr(),
                enabled_extension_count: enabled_extension_names.len() as u32,
                pp_enabled_extension_names: enabled_extension_names.as_ptr(),
                p_enabled_features: &self.enabled_features as *const _,
            },
            queue_create_infos,
            enabled_layer_names,
            enabled_extension_names,
            phantom: marker::PhantomData,
        }
    }
}

impl<'a> DeviceCreateInfoRaw<'a> {
    /// Retrieve a reference to `vk::DeviceCreateInfo`.
    pub fn device_create_info(&self) -> &vk::DeviceCreateInfo {
        &self.info
    }
}

impl<'a> ops::Deref for DeviceCreateInfoRaw<'a> {
    type Target = vk::DeviceCreateInfo;

    fn deref(&self) -> &Self::Target {
        self.device_create_info()
    }
}

/// Memory-safe wrapper for `ApplicationInfo`.
#[derive(Debug, Clone)]
pub struct ApplicationInfo {
    pub application_name: Option<ffi::CString>,
    pub application_version: u32,
    pub engine_name: Option<ffi::CString>,
    pub engine_version: u32,
    pub api_version: u32,
}

impl ApplicationInfo {
    pub fn as_raw(&self) -> vk::ApplicationInfo {
        vk::ApplicationInfo {
            s_type: vk::StructureType::ApplicationInfo,
            p_next: ptr::null(),
            p_application_name: self.application_name
                .as_ref()
                .map(|x| x.as_ptr())
                .unwrap_or_else(ptr::null),
            application_version: self.application_version,
            p_engine_name: self.engine_name
                .as_ref()
                .map(|x| x.as_ptr())
                .unwrap_or_else(ptr::null),
            engine_version: self.engine_version,
            api_version: self.api_version,
        }
    }
}

impl Default for ApplicationInfo {
    fn default() -> Self {
        Self {
            application_name: None,
            application_version: 0,
            engine_name: None,
            engine_version: 0,
            api_version: ((1 << 22) | (0 << 12) | (0)) as u32,
        }
    }
}
