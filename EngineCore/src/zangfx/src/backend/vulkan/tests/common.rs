//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#[macro_use]
extern crate ash;
extern crate zangfx_base as base;
#[macro_use]
extern crate zangfx_test;
extern crate zangfx_vulkan as backend;

use ash::extensions::DebugReport;
use ash::version::*;
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::ptr::{null, null_mut};

struct TestDriver;

struct UniqueInstance(ash::Instance<V1_0>);

impl Drop for UniqueInstance {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_instance(None);
        }
    }
}

impl Deref for UniqueInstance {
    type Target = ash::Instance<V1_0>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct UniqueDevice(ash::Device<V1_0>);

impl Drop for UniqueDevice {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_device(None);
        }
    }
}

impl Deref for UniqueDevice {
    type Target = ash::Device<V1_0>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct DebugReportScope(DebugReport, ash::vk::DebugReportCallbackEXT);

impl DebugReportScope {
    fn new(e: &ash::Entry<V1_0>, i: &ash::Instance<V1_0>) -> Self {
        let debug_report_loader = DebugReport::new(e, i).unwrap();

        unsafe extern "system" fn vulkan_debug_callback(
            _: ash::vk::DebugReportFlagsEXT,
            _: ash::vk::DebugReportObjectTypeEXT,
            _: ash::vk::uint64_t,
            _: ash::vk::size_t,
            _: ash::vk::int32_t,
            _: *const ash::vk::c_char,
            p_message: *const ash::vk::c_char,
            _: *mut ash::vk::c_void,
        ) -> u32 {
            println!(" <debug report> {:?}", CStr::from_ptr(p_message));
            1
        }

        let debug_info = ash::vk::DebugReportCallbackCreateInfoEXT {
            s_type: ash::vk::StructureType::DebugReportCallbackCreateInfoExt,
            p_next: null(),
            flags: ash::vk::DEBUG_REPORT_ERROR_BIT_EXT | ash::vk::DEBUG_REPORT_WARNING_BIT_EXT,
            pfn_callback: vulkan_debug_callback,
            p_user_data: null_mut(),
        };

        let cb = unsafe { debug_report_loader.create_debug_report_callback_ext(&debug_info, None) }
            .unwrap();
        DebugReportScope(debug_report_loader, cb)
    }
}

impl Drop for DebugReportScope {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_debug_report_callback_ext(self.1, None);
        }
    }
}

impl zangfx_test::backend_tests::TestDriver for TestDriver {
    fn for_each_device(&self, runner: &mut FnMut(&base::device::Device)) {
        unsafe {
            let entry = match ash::Entry::<V1_0>::new() {
                Ok(entry) => entry,
                Err(err) => {
                    println!(
                        "Failed to load the Vulkan runtime. Skipping the test.: {:?}",
                        err
                    );
                    return;
                }
            };

            let layer_props = entry.enumerate_instance_layer_properties().unwrap();
            let ext_props = entry.enumerate_instance_extension_properties().unwrap();

            let validation_layer_name =
                CString::new("VK_LAYER_LUNARG_standard_validation").unwrap();

            let mut layers = Vec::new();
            let mut extensions = Vec::new();

            if layer_props
                .iter()
                .any(|p| CStr::from_ptr(p.layer_name.as_ptr()) == validation_layer_name.as_c_str())
            {
                layers.push(validation_layer_name.as_ptr());
            } else {
                println!(
                    "Warning: Layer '{:?}' is unavailable",
                    validation_layer_name
                );
            }

            let has_debug_report = ext_props
                .iter()
                .any(|p| CStr::from_ptr(p.extension_name.as_ptr()) == DebugReport::name());
            if has_debug_report {
                extensions.push(DebugReport::name().as_ptr());
            } else {
                println!(
                    "Warning: Extension '{:?}' is unavailable",
                    DebugReport::name()
                );
            }

            let instance: UniqueInstance = entry
                .create_instance(
                    &ash::vk::InstanceCreateInfo {
                        s_type: ash::vk::StructureType::InstanceCreateInfo,
                        p_next: null(),
                        flags: ash::vk::InstanceCreateFlags::empty(),
                        p_application_info: &ash::vk::ApplicationInfo {
                            s_type: ash::vk::StructureType::ApplicationInfo,
                            p_next: null(),
                            p_application_name: b"ZanGFX Test Suite\0".as_ptr() as *const _,
                            application_version: 1,
                            p_engine_name: null(),
                            engine_version: 0,
                            api_version: vk_make_version!(1, 0, 0),
                        },
                        enabled_layer_count: layers.len() as u32,
                        pp_enabled_layer_names: layers.as_ptr() as *const _,
                        enabled_extension_count: extensions.len() as u32,
                        pp_enabled_extension_names: extensions.as_ptr() as *const _,
                    },
                    None,
                )
                .map(UniqueInstance)
                .expect("Failed to create a Vulkan instance.");

            let _debug_report = if has_debug_report {
                Some(DebugReportScope::new(&entry, &instance))
            } else {
                None
            };

            let phys_devices = instance.enumerate_physical_devices().unwrap();
            for &phys_device in phys_devices.iter() {
                let prop = instance.get_physical_device_properties(phys_device);
                println!();
                println!(
                    "[[Physical device '{:?}']]",
                    CStr::from_ptr(prop.device_name.as_ptr())
                );

                let available_features = instance.get_physical_device_features(phys_device);

                if available_features.robust_buffer_access == ash::vk::VK_FALSE {
                    println!("Warning: Feature 'robust_buffer_access' is unavailable");
                }

                let enabled_features = ash::vk::PhysicalDeviceFeatures {
                    robust_buffer_access: available_features.robust_buffer_access,
                    ..Default::default()
                };

                let info = backend::limits::DeviceInfo::from_physical_device(
                    &instance,
                    phys_device,
                    &enabled_features,
                ).unwrap();

                // Allocate some queues
                use std::cmp::min;
                let queues = info
                    .queue_families
                    .iter()
                    .enumerate()
                    .map(|(i, prop)| ash::vk::DeviceQueueCreateInfo {
                        s_type: ash::vk::StructureType::DeviceQueueCreateInfo,
                        p_next: null(),
                        flags: ash::vk::DeviceQueueCreateFlags::empty(),
                        queue_family_index: i as u32,
                        queue_count: min(2, prop.count) as u32,
                        p_queue_priorities: [0.5f32, 0.5f32].as_ptr(),
                    })
                    .collect::<Vec<_>>();

                let mut config = backend::limits::DeviceConfig::new();

                for queue_ci in queues.iter() {
                    for i in 0..queue_ci.queue_count {
                        config.queues.push((queue_ci.queue_family_index, i));
                    }
                }

                let device = instance
                    .create_device(
                        phys_device,
                        &ash::vk::DeviceCreateInfo {
                            s_type: ash::vk::StructureType::DeviceCreateInfo,
                            p_next: null(),
                            flags: ash::vk::DeviceCreateFlags::empty(),
                            queue_create_info_count: queues.len() as u32,
                            p_queue_create_infos: queues.as_ptr(),
                            enabled_layer_count: 0,
                            pp_enabled_layer_names: null(),
                            enabled_extension_count: 0,
                            pp_enabled_extension_names: null(),
                            p_enabled_features: &enabled_features,
                        },
                        None,
                    )
                    .map(UniqueDevice)
                    .expect("Failed to create a Vulkan device.");

                let gfx_device =
                    backend::device::Device::new(ash::Device::clone(&device), info, config)
                        .expect("Failed to create a ZanGFX device.");
                runner(&gfx_device);
            }
        }
    }
}

zangfx_generate_backend_tests!(TestDriver);
