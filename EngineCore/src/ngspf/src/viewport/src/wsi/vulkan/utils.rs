//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::ash::{self, version::*, vk};
use super::be;
use std::collections::HashSet;
use std::ffi::{CStr, CString};
use zangfx::backends::vulkan::translate_generic_error;
use zangfx::{base::Device, common::Error};

use super::smartptr::{UniqueDevice, UniqueInstance};
use super::AppInfo;

pub fn vk_device_from_gfx(device: &Device) -> &ash::Device<V1_0> {
    let be_device: &be::device::Device = device.query_ref().unwrap();
    be_device.vk_device()
}

pub fn translate_generic_error_unwrap(result: vk::Result) -> Error {
    translate_generic_error(result).unwrap()
}

pub struct InstanceBuilder<'a> {
    entry: &'a ash::Entry<V1_0>,
    supported_layers: Vec<(String, u32)>,
    supported_extensions: Vec<(String, u32)>,
    enabled_layers: HashSet<String>,
    enabled_extensions: HashSet<String>,
}

impl<'a> InstanceBuilder<'a> {
    pub fn new(entry: &'a ash::Entry<V1_0>) -> Result<Self, vk::Result> {
        let layer_props = entry.enumerate_instance_layer_properties()?;
        let ext_props = entry.enumerate_instance_extension_properties()?;

        let supported_layers: Vec<_> = layer_props
            .iter()
            .map(|e| {
                let name = unsafe { CStr::from_ptr(e.layer_name.as_ptr()) };
                (name.to_str().unwrap().to_owned(), e.spec_version)
            })
            .collect();
        let supported_extensions: Vec<_> = ext_props
            .iter()
            .map(|e| {
                let name = unsafe { CStr::from_ptr(e.extension_name.as_ptr()) };
                (name.to_str().unwrap().to_owned(), e.spec_version)
            })
            .collect();

        Ok(Self {
            entry,
            supported_layers,
            supported_extensions,
            enabled_layers: HashSet::new(),
            enabled_extensions: HashSet::new(),
        })
    }

    pub fn supports_layer(&self, name: &str) -> bool {
        self.supported_layers.iter().any(|x| x.0 == name)
    }

    pub fn supports_extension(&self, name: &str) -> bool {
        self.supported_extensions.iter().any(|x| x.0 == name)
    }

    pub fn enable_layer(&mut self, name: &str) {
        assert!(
            self.supports_layer(name),
            "Layer '{}' is not supported",
            name
        );
        self.enabled_layers.insert(name.to_owned());
    }

    pub fn enable_extension(&mut self, name: &str) {
        assert!(
            self.supports_extension(name),
            "Extension '{}' is not supported",
            name
        );
        self.enabled_extensions.insert(name.to_owned());
    }

    pub fn build(&self, app_info: &AppInfo) -> Result<UniqueInstance, ash::InstanceError> {
        let layers: Vec<_> = self.enabled_layers
            .iter()
            .map(|x| CString::new(x.as_str()).unwrap())
            .collect();
        let extensions: Vec<_> = self.enabled_extensions
            .iter()
            .map(|x| CString::new(x.as_str()).unwrap())
            .collect();

        let layers: Vec<_> = layers.iter().map(|x| x.as_ptr()).collect();
        let extensions: Vec<_> = extensions.iter().map(|x| x.as_ptr()).collect();

        macro_rules! vk_make_version {
            ($major:expr, $minor:expr, $patch:expr) => {
                (($major as u32) << 22) | (($minor as u32) << 12) | $patch as u32
            };
        }

        let application_name = CString::new(app_info.name).unwrap();

        let application_info = vk::ApplicationInfo {
            s_type: vk::StructureType::ApplicationInfo,
            p_next: ::null(),
            p_application_name: application_name.as_ptr(),
            application_version: app_info.version,
            p_engine_name: b"Nightingales\0".as_ptr() as *const _,
            engine_version: 0,
            api_version: vk_make_version!(1, 0, 0),
        };

        unsafe {
            self.entry
                .create_instance(
                    &vk::InstanceCreateInfo {
                        s_type: vk::StructureType::InstanceCreateInfo,
                        p_next: ::null(),
                        flags: vk::InstanceCreateFlags::empty(),
                        p_application_info: &application_info,
                        enabled_layer_count: layers.len() as u32,
                        pp_enabled_layer_names: layers.as_ptr() as *const _,
                        enabled_extension_count: extensions.len() as u32,
                        pp_enabled_extension_names: extensions.as_ptr() as *const _,
                    },
                    None,
                )
                .map(UniqueInstance)
        }
    }
}

pub struct DeviceBuilder<'a> {
    phys_device: vk::PhysicalDevice,
    instance: &'a ash::Instance<V1_0>,
    supported_extensions: Vec<(String, u32)>,
    enabled_extensions: HashSet<String>,
}

impl<'a> DeviceBuilder<'a> {
    pub fn new(
        instance: &'a ash::Instance<V1_0>,
        phys_device: vk::PhysicalDevice,
    ) -> Result<Self, vk::Result> {
        let ext_props = instance.enumerate_device_extension_properties(phys_device)?;

        let supported_extensions: Vec<_> = ext_props
            .iter()
            .map(|e| {
                let name = unsafe { CStr::from_ptr(e.extension_name.as_ptr()) };
                (name.to_str().unwrap().to_owned(), e.spec_version)
            })
            .collect();

        Ok(Self {
            phys_device,
            instance,
            supported_extensions,
            enabled_extensions: HashSet::new(),
        })
    }

    pub fn supports_extension(&self, name: &str) -> bool {
        self.supported_extensions.iter().any(|x| x.0 == name)
    }

    pub fn enable_extension(&mut self, name: &str) {
        assert!(
            self.supports_extension(name),
            "Device extension '{}' is not supported",
            name
        );
        self.enabled_extensions.insert(name.to_owned());
    }

    pub fn build(
        &self,
        queue_create_infos: &[vk::DeviceQueueCreateInfo],
        enabled_features: &vk::PhysicalDeviceFeatures,
    ) -> Result<UniqueDevice, ash::DeviceError> {
        let extensions: Vec<_> = self.enabled_extensions
            .iter()
            .map(|x| CString::new(x.as_str()).unwrap())
            .collect();

        let extensions: Vec<_> = extensions.iter().map(|x| x.as_ptr()).collect();

        unsafe {
            self.instance
                .create_device(
                    self.phys_device,
                    &vk::DeviceCreateInfo {
                        s_type: vk::StructureType::DeviceCreateInfo,
                        p_next: ::null(),
                        flags: vk::DeviceCreateFlags::empty(),
                        queue_create_info_count: queue_create_infos.len() as u32,
                        p_queue_create_infos: queue_create_infos.as_ptr(),
                        enabled_layer_count: 0,
                        pp_enabled_layer_names: ::null(),
                        enabled_extension_count: extensions.len() as u32,
                        pp_enabled_extension_names: extensions.as_ptr() as *const _,
                        p_enabled_features: enabled_features,
                    },
                    None,
                )
                .map(UniqueDevice)
        }
    }
}
