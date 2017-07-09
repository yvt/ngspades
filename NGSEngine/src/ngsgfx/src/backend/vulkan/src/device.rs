//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, ash};
use ash::vk;
use ash::version::{InstanceV1_0, V1_0};

use std::sync::Arc;
use std::{ptr, ffi};
use std::collections::{HashSet, VecDeque};

use {DeviceRef, OwnedDeviceRef, translate_generic_error_unwrap};
use imp::{Backend, CommandQueue, DeviceCapabilities, EngineQueueMappings, EngineQueueMapping};
use ll::{DeviceCreateInfo, DeviceQueueCreateInfo};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DeviceFeature {
    DepthBounds,
    CubeArrayImage,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DeviceElaborateError {
    /// The given physical device does not support graphics operations.
    NoGraphicsQueueFamily,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum DeviceBuildError {
    ElaborateError(DeviceElaborateError),
    GenericError(core::GenericError),
    LoadError(Vec<&'static str>),
    InitializationFailed,
    ExtensionNotPresent,
    FeatureNotPresent,
    TooManyObjects,
}

#[derive(Clone)]
pub struct DeviceBuilder<'a> {
    instance: &'a ash::Instance<V1_0>,
    physical_device: vk::PhysicalDevice,
    features: vk::PhysicalDeviceFeatures,
    supported_features: vk::PhysicalDeviceFeatures,
    layers: VecDeque<ffi::CString>,
    extensions: HashSet<ffi::CString>,
    supported_extensions: HashSet<ffi::CString>,
}

impl<'a> DeviceBuilder<'a> {
    /// Constructs a new `DeviceBuilder`.
    ///
    /// - The specified Vulkan instance pointed at by `instance` must be valid
    ///   and outlive the created `DeviceBuilder`.
    /// - The specified `physical_device` must be valid.
    pub unsafe fn new(
        instance: &'a ash::Instance<V1_0>,
        physical_device: vk::PhysicalDevice,
    ) -> Self {
        let supported_features = instance.get_physical_device_features(physical_device);
        let ext_props = instance.enumerate_device_extension_properties(physical_device).unwrap();
        let mut supported_extensions = HashSet::new();

        for ext in ext_props.iter() {
            let name = unsafe { ffi::CStr::from_ptr(ext.extension_name.as_ptr()) };
            supported_extensions.insert(name.to_owned());
        }

        Self {
            instance,
            physical_device,
            features: Default::default(),
            supported_features,
            layers: VecDeque::new(),
            extensions: HashSet::new(),
            supported_extensions,
        }
    }

    /// Enable the specified feature if available.
    ///
    /// Returns whether the specified feature is available.
    pub fn enable_feature(&mut self, feature: DeviceFeature) -> bool {
        match feature {
            DeviceFeature::DepthBounds => {
                self.features.depth_bounds = self.supported_features.depth_bounds;
                self.features.depth_bounds != vk::VK_FALSE
            }
            DeviceFeature::CubeArrayImage => {
                self.features.image_cube_array = self.supported_features.image_cube_array;
                self.features.image_cube_array != vk::VK_FALSE
            }
        }
    }

    /// Attempts to enable all features.
    pub fn enable_all_features(&mut self) -> &mut Self {
        self.enable_feature(DeviceFeature::DepthBounds);
        self.enable_feature(DeviceFeature::CubeArrayImage);
        self
    }

    /// Enable the specified extension if available.
    ///
    /// Returns whether the specified extension is available.
    pub fn enable_extension(&mut self, name: &str) -> bool {
        let cname = ffi::CString::new(name);
        if let Ok(cname) = cname {
            if self.supported_extensions.contains(&cname) {
                self.extensions.insert(cname);
                true
            } else {
                false
            }
        } else {
            // No extension has a null character in its name
            false
        }
    }

    pub fn supports_extension(&self, name: &str) -> bool {
        let cname = ffi::CString::new(name);
        cname.map(|n| self.supported_extensions.contains(&n)).unwrap_or(false)
    }

    /// Appends the specified layer.
    pub fn push_back_layer(&mut self, name: &str) {
        self.layers.push_back(ffi::CString::new(name).unwrap());
    }

    /// Prepends the specified layer.
    pub fn push_front_layer(&mut self, name: &str) {
        self.layers.push_front(ffi::CString::new(name).unwrap());
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }

    /// Retrieve some structures required to construct a `Device`.
    ///
    /// You can this function to perform modification or inspection on these
    /// structure.
    pub fn info(&self) -> Result<(DeviceCreateInfo, EngineQueueMappings, DeviceCapabilities), DeviceElaborateError> {
        let qf_props: Vec<vk::QueueFamilyProperties> =
            self.instance.get_physical_device_queue_family_properties(self.physical_device);
        let mut used_count = vec![0; qf_props.len()];

        // Universal engine: Choose the first queue family supporting all kind of operations
        let mut universal_fam_index = None;
        for (i, qfp) in qf_props.iter().enumerate() {
            if qfp.queue_flags.subset(vk::QUEUE_GRAPHICS_BIT | vk::QUEUE_COMPUTE_BIT) {
                universal_fam_index = Some(i);
                break;
            }
        }
        let universal_fam_index = universal_fam_index.ok_or(DeviceElaborateError::NoGraphicsQueueFamily)?;
        let universal_index = used_count[universal_fam_index];
        used_count[universal_fam_index] += 1;

        // Compute engine: Choose the first queue family supporting compute operations but
        //                 no graphics operations.
        //                 If none found, choose the first queue family supporting compute operations
        //                 and its limit on the number of queues have not been reached yet.
        //                 If none found yet, use the same queue as the universal engine.
        let mut compute_fam_index = None;
        for (i, qfp) in qf_props.iter().enumerate() {
            if qfp.queue_flags.intersects(vk::QUEUE_COMPUTE_BIT) &&
                !qfp.queue_flags.intersects(vk::QUEUE_GRAPHICS_BIT)
            {
                compute_fam_index = Some(i);
                break;
            }
        }
        if compute_fam_index.is_none() {
            for (i, qfp) in qf_props.iter().enumerate() {
                if qfp.queue_flags.intersects(vk::QUEUE_COMPUTE_BIT) &&
                    used_count[i] < qfp.queue_count
                {
                    compute_fam_index = Some(i);
                    break;
                }
            }
        }
        let (compute_fam_index, compute_index) = if let Some(i) = compute_fam_index {
            used_count[i] += 1;
            (i, used_count[i] - 1)
        } else {
            (universal_fam_index, universal_index)
        };

        // Copy engine: Choose the first queue family supporting transfer operations but
        //              no other operations.
        //              If none found, choose the first queue family supporting transfer operations
        //              and its limit on the number of queues have not been reached yet.
        //              If none found yet, use the same queue as the universal engine.
        let mut copy_fam_index = None;
        for (i, qfp) in qf_props.iter().enumerate() {
            if qfp.queue_flags.intersects(vk::QUEUE_TRANSFER_BIT) &&
                !qfp.queue_flags.intersects(vk::QUEUE_GRAPHICS_BIT | vk::QUEUE_COMPUTE_BIT)
            {
                copy_fam_index = Some(i);
                break;
            }
        }
        if copy_fam_index.is_none() {
            for (i, qfp) in qf_props.iter().enumerate() {
                // `QUEUE_GRAPHICS_BIT` and `QUEUE_COMPUTE_BIT` imply `QUEUE_TRANSFER_BIT`
                // (Vulkan 1.0 Spec 4.1. "Physical Devices" Note)
                if qfp.queue_flags.intersects(vk::QUEUE_TRANSFER_BIT | vk::QUEUE_GRAPHICS_BIT | vk::QUEUE_COMPUTE_BIT) &&
                    used_count[i] < qfp.queue_count
                {
                    copy_fam_index = Some(i);
                    break;
                }
            }
        }
        let (copy_fam_index, copy_index) = if let Some(i) = copy_fam_index {
            used_count[i] += 1;
            (i, used_count[i] - 1)
        } else {
            (universal_fam_index, universal_index)
        };

        let eqm = EngineQueueMappings {
            universal: EngineQueueMapping {
                queue_family_index: universal_fam_index as u32,
                queue_index: universal_index,
            },
            compute: EngineQueueMapping {
                queue_family_index: compute_fam_index as u32,
                queue_index: compute_index,
            },
            copy: EngineQueueMapping {
                queue_family_index: copy_fam_index as u32,
                queue_index: copy_index,
            },
        };

        let mut queue_create_infos = Vec::new();
        for (fam_idx, &count) in used_count.iter().enumerate().filter(|&(_,&c)|c>0) {
            queue_create_infos.push(DeviceQueueCreateInfo{
                p_next: ptr::null(),
                flags: vk::DeviceQueueCreateFlags::empty(),
                queue_family_index: fam_idx as u32,
                queue_priorities: vec![0.5f32; count as usize],
            });
        }

        let cap = DeviceCapabilities::new(self.instance, self.physical_device, &self.features);

        // TODO: enable debug markers (VK_EXT_debug_marker) somehow

        Ok((
            DeviceCreateInfo {
                p_next: ptr::null(),
                flags: vk::DeviceCreateFlags::empty(), // reserved
                enabled_features: self.features.clone(),
                queue_create_infos,
                enabled_layer_names: Vec::from(self.layers.clone()),
                enabled_extension_names: self.extensions.iter().map(Clone::clone).collect(),
            },
            eqm,
            cap,
        ))
    }

    /// Constructs a `Device<OwnedDeviceRef>`.
    pub unsafe fn build(&self) -> Result<Device<OwnedDeviceRef>, DeviceBuildError> {
        let (dci, eqm, dc) = self.info()
            .map_err(DeviceBuildError::ElaborateError)?;
        let inst = self.instance;
        let dci_raw = dci.as_raw();
        let dev = unsafe {
            inst.create_device(self.physical_device, &dci_raw, None)
        }.map_err(|e| {
            match e {
                ash::DeviceError::LoadError(errors) =>
                    DeviceBuildError::LoadError(errors),
                ash::DeviceError::VkError(vk::Result::ErrorInitializationFailed) =>
                    DeviceBuildError::InitializationFailed,
                ash::DeviceError::VkError(vk::Result::ErrorExtensionNotPresent) =>
                    DeviceBuildError::ExtensionNotPresent,
                ash::DeviceError::VkError(vk::Result::ErrorFeatureNotPresent) =>
                    DeviceBuildError::FeatureNotPresent,
                ash::DeviceError::VkError(vk::Result::ErrorTooManyObjects) =>
                    DeviceBuildError::TooManyObjects,
                ash::DeviceError::VkError(e) =>
                    DeviceBuildError::GenericError(translate_generic_error_unwrap(e))
            }
        })?;
        let dev_ref = OwnedDeviceRef::from_raw(dev);
        Ok(Device::new(dev_ref, eqm, dc))
    }
}

pub struct Device<T: DeviceRef> {
    data: Arc<DeviceData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for Device<T> => data
}

#[derive(Debug)]
pub(crate) struct DeviceData<T: DeviceRef> {
    device_ref: T,
    cap: DeviceCapabilities,
    queue_mappings: EngineQueueMappings,
}

impl<T: DeviceRef> core::Device<Backend<T>> for Device<T> {
    fn main_queue(&self) -> &CommandQueue<T> {
        unimplemented!()
    }
    fn factory(&self) -> &Device<T> {
        &self
    }
    fn capabilities(&self) -> &DeviceCapabilities {
        &self.data.cap
    }
}

impl<T: DeviceRef> Device<T> {
    pub fn new(
        device_ref: T,
        queue_mappings: EngineQueueMappings,
        cap: DeviceCapabilities,
    ) -> Self {
        Device {
            data: Arc::new(DeviceData{
                device_ref,
                cap,
                queue_mappings,
            }),
        }
    }
    pub(crate) fn data(&self) -> &DeviceData<T> {
        &*self.data
    }
    pub(crate) fn device_ref(&self) -> &T {
        &self.data.device_ref
    }
    pub(crate) fn capabilities(&self) -> &DeviceCapabilities {
        &self.data.cap
    }
}
