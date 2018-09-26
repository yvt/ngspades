//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![warn(rust_2018_idioms)]
#![feature(test)]

// Despite the compiler warning, we still need this for benchmarking
#![allow(rust_2018_idioms)]
extern crate test;

use zangfx_base as base;
use zangfx_vulkan as backend;

use zangfx_test::zangfx_generate_backend_benches;

use ash::version::*;
use ash::vk_make_version;
use std::ffi::CStr;
use std::ops::Deref;
use std::ptr::null;
use std::sync::Arc;

struct BenchDriver;

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

impl zangfx_test::backend_benches::BenchDriver for BenchDriver {
    fn choose_device(&self, runner: &mut dyn FnMut(&base::device::DeviceRef)) {
        unsafe {
            let entry = match ash::Entry::<V1_0>::new() {
                Ok(entry) => entry,
                Err(err) => {
                    panic!("Failed to load the Vulkan runtime.: {:?}", err);
                }
            };

            let layers = Vec::new();
            let extensions = Vec::new();

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
                ).map(UniqueInstance)
                .expect("Failed to create a Vulkan instance.");

            let phys_devices = instance.enumerate_physical_devices().unwrap();
            for &phys_device in phys_devices.iter() {
                let prop = instance.get_physical_device_properties(phys_device);
                println!();
                println!(
                    "[[Physical device '{:?}']]",
                    CStr::from_ptr(prop.device_name.as_ptr())
                );

                let enabled_features = ash::vk::PhysicalDeviceFeatures {
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
                    }).collect::<Vec<_>>();

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
                    ).map(UniqueDevice)
                    .expect("Failed to create a Vulkan device.");

                let gfx_device =
                    backend::device::Device::new(ash::Device::clone(&device), info, config)
                        .expect("Failed to create a ZanGFX device.");

                let gfx_device_ref: base::DeviceRef = Arc::new(gfx_device);

                runner(&gfx_device_ref);

                backend::device::Device::teardown_ref(&mut { gfx_device_ref });

                break;
            }
        }
    }
}

zangfx_generate_backend_benches!(BenchDriver);
