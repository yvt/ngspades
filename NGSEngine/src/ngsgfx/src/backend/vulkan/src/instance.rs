//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::{self, vk};
use ash::version::{V1_0, InstanceV1_0, EntryV1_0};
use std::sync::Arc;
use std::{mem, ops, ffi, ptr, fmt};
use std::collections::{VecDeque, HashSet, HashMap};

use imp::{ManagedEnvironment, Device, EngineQueueMappings, EngineQueueMapping, DeviceCapabilities};
use ll::{DeviceCreateInfo, DeviceQueueCreateInfo, ApplicationInfo};
use {translate_generic_error_unwrap, RefEqArc, OwnedInstanceRef, AshInstance, ManagedDeviceRef};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InstanceBuildError {
    GenericError(core::GenericError),
    LoadError(Vec<&'static str>),
    InitializationFailed,
    ExtensionNotPresent,
    LayerNotPresent,
    IncompatibleDriver,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DeviceFeature {
    DepthBounds,
    CubeArrayImage,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum DeviceElaborateError {
    /// The given physical device does not support graphics operations.
    NoGraphicsQueueFamily,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum DeviceBuildError {
    GenericError(core::GenericError),
    LoadError(Vec<&'static str>),
    InitializationFailed,
    ExtensionNotPresent,
    FeatureNotPresent,
    TooManyObjects,
}

pub struct InstanceBuilder {
    entry: ash::Entry<V1_0>,
    app_info: ApplicationInfo,
    layers: VecDeque<ffi::CString>,
    extensions: HashSet<ffi::CString>,
    supported_layers: HashMap<ffi::CString, u32>,
    supported_extensions: HashMap<ffi::CString, u32>,
}

impl fmt::Debug for InstanceBuilder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InstanceBuilder")
            .field("app_info", &self.app_info)
            .field("layers", &self.layers)
            .field("extensions", &self.extensions)
            .field("supported_layers", &self.supported_layers)
            .field("supported_extensions", &self.supported_extensions)
            .finish()
    }
}

impl core::InstanceBuilder<ManagedEnvironment> for InstanceBuilder {
    type BuildError = InstanceBuildError;
    type InitializationError = ash::LoadingError;

    fn new() -> Result<Self, Self::InitializationError> {
        let entry = ash::Entry::new()?;
        let layer_props = entry.enumerate_instance_layer_properties().unwrap();
        let ext_props = entry.enumerate_instance_extension_properties().unwrap();
        let supported_layers = layer_props
            .iter()
            .map(|e| {
                let name = unsafe { ffi::CStr::from_ptr(e.layer_name.as_ptr()) };
                (name.to_owned(), e.spec_version)
            })
            .collect();
        let supported_extensions = ext_props
            .iter()
            .map(|e| {
                let name = unsafe { ffi::CStr::from_ptr(e.extension_name.as_ptr()) };
                (name.to_owned(), e.spec_version)
            })
            .collect();

        Ok(Self {
            entry,
            app_info: ApplicationInfo::default(),
            layers: VecDeque::new(),
            extensions: HashSet::new(),
            supported_layers,
            supported_extensions,
        })
    }
    fn build(&self) -> Result<Instance, Self::BuildError> {
        let layers: Vec<_> = self.layers.iter().map(|n| n.as_ptr()).collect();
        let exts: Vec<_> = self.extensions.iter().map(|n| n.as_ptr()).collect();
        let inst = self.entry
            .create_instance(
                &vk::InstanceCreateInfo {
                    s_type: vk::StructureType::InstanceCreateInfo,
                    p_next: ptr::null(),
                    flags: vk::InstanceCreateFlags::empty(),
                    p_application_info: &self.app_info.as_raw() as *const _,
                    enabled_layer_count: layers.len() as u32,
                    pp_enabled_layer_names: layers.as_ptr() as *const _,
                    enabled_extension_count: exts.len() as u32,
                    pp_enabled_extension_names: exts.as_ptr() as *const _,
                },
                None,
            )
            .map_err(|e| match e {
                ash::InstanceError::LoadError(errors) => InstanceBuildError::LoadError(errors),
                ash::InstanceError::VkError(vk::Result::ErrorInitializationFailed) => {
                    InstanceBuildError::InitializationFailed
                }
                ash::InstanceError::VkError(vk::Result::ErrorExtensionNotPresent) => {
                    InstanceBuildError::ExtensionNotPresent
                }
                ash::InstanceError::VkError(vk::Result::ErrorLayerNotPresent) => {
                    InstanceBuildError::LayerNotPresent
                }
                ash::InstanceError::VkError(vk::Result::ErrorIncompatibleDriver) => {
                    InstanceBuildError::IncompatibleDriver
                }
                ash::InstanceError::VkError(e) => InstanceBuildError::GenericError(
                    translate_generic_error_unwrap(e),
                ),
            })?;
        Ok(Instance::from_raw(inst))
    }
}

impl InstanceBuilder {
    pub fn set_app_info(&mut self, app_info: ApplicationInfo) -> &mut Self {
        self.app_info = app_info;
        self
    }

    /// Enable the specified extension if available.
    ///
    /// Returns whether the specified extension is available.
    pub fn enable_extension(&mut self, name: &str) -> bool {
        let cname = ffi::CString::new(name);
        if let Ok(cname) = cname {
            if self.supported_extensions.contains_key(&cname) {
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

    pub fn extension_version(&self, name: &str) -> Option<u32> {
        let cname = ffi::CString::new(name);
        cname
            .ok()
            .and_then(|n| self.supported_extensions.get(&n))
            .map(|&ver| ver)
    }

    pub fn supports_extension(&self, name: &str) -> bool {
        self.extension_version(name).is_some()
    }

    /// Appends the standard validation layers (`VK_LAYER_LUNARG_standard_validation`).
    pub fn push_back_standard_validation_layer(&mut self) {
        self.push_back_layer("VK_LAYER_LUNARG_standard_validation");
    }

    /// Appends the specified layer if available.
    pub fn push_back_layer(&mut self, name: &str) -> bool {
        let cname = ffi::CString::new(name);
        if let Ok(cname) = cname {
            if self.supported_layers.contains_key(&cname) {
                self.layers.push_back(ffi::CString::new(name).unwrap());
                true
            } else {
                false
            }
        } else {
            // No layer has a null character in its name (duh!)
            false
        }
    }

    /// Prepends the specified layer if available.
    pub fn push_front_layer(&mut self, name: &str) -> bool {
        let cname = ffi::CString::new(name);
        if let Ok(cname) = cname {
            if self.supported_layers.contains_key(&cname) {
                self.layers.push_front(ffi::CString::new(name).unwrap());
                true
            } else {
                false
            }
        } else {
            // No layer has a null character in its name (duh!)
            false
        }
    }

    pub fn layer_version(&self, name: &str) -> Option<u32> {
        let cname = ffi::CString::new(name);
        cname.ok().and_then(|n| self.supported_layers.get(&n)).map(
            |&ver| ver,
        )
    }

    pub fn supports_layer(&self, name: &str) -> bool {
        self.layer_version(name).is_some()
    }
}

pub(crate) struct UniqueInstance(AshInstance);
impl UniqueInstance {
    fn take(this: Self) -> AshInstance {
        let ret = this.0.clone();
        mem::forget(this);
        ret
    }
}
impl Drop for UniqueInstance {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_instance(None);
        }
    }
}
impl ops::Deref for UniqueInstance {
    type Target = AshInstance;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl fmt::Debug for UniqueInstance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("UniqueInstance")
            .field(&self.0.handle())
            .finish()
    }
}

pub struct Instance {
    instance: Arc<UniqueInstance>,
    adapters: Vec<Adapter>,
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Instance")
            .field("adapters", &self.adapters)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Adapter(RefEqArc<AdapterData>);

#[derive(Debug)]
struct AdapterData {
    physical_device: vk::PhysicalDevice,
    name: String,
    eqm: EngineQueueMappings,
}

impl core::Adapter for Adapter {
    fn name(&self) -> &str {
        "System default adapter"
    }
}

impl AdapterData {
    /// Constructs an `Adapter` from `vk::PhysicalDevice`.
    ///
    /// Returns `Err` if the specified physical device does not support festures
    /// required by NgsGFX.
    fn new(
        instance: &AshInstance,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self, DeviceElaborateError> {
        let props = instance.get_physical_device_properties(physical_device);
        let name = unsafe { ffi::CStr::from_ptr(props.device_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();

        let qf_props: Vec<vk::QueueFamilyProperties> =
            instance.get_physical_device_queue_family_properties(physical_device);
        let mut used_count = vec![0; qf_props.len()];

        // Universal engine: Choose the first queue family supporting all kind of operations
        let mut universal_fam_index = None;
        for (i, qfp) in qf_props.iter().enumerate() {
            if qfp.queue_flags.subset(
                vk::QUEUE_GRAPHICS_BIT |
                    vk::QUEUE_COMPUTE_BIT,
            )
            {
                universal_fam_index = Some(i);
                break;
            }
        }
        let universal_fam_index = universal_fam_index.ok_or(
            DeviceElaborateError::NoGraphicsQueueFamily,
        )?;
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
                !qfp.queue_flags.intersects(
                    vk::QUEUE_GRAPHICS_BIT |
                        vk::QUEUE_COMPUTE_BIT,
                )
            {
                copy_fam_index = Some(i);
                break;
            }
        }
        if copy_fam_index.is_none() {
            for (i, qfp) in qf_props.iter().enumerate() {
                // `QUEUE_GRAPHICS_BIT` and `QUEUE_COMPUTE_BIT` imply `QUEUE_TRANSFER_BIT`
                // (Vulkan 1.0 Spec 4.1. "Physical Devices" Note)
                if qfp.queue_flags.intersects(
                    vk::QUEUE_TRANSFER_BIT | vk::QUEUE_GRAPHICS_BIT |
                        vk::QUEUE_COMPUTE_BIT,
                ) && used_count[i] < qfp.queue_count
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

        Ok(Self {
            physical_device,
            name,
            eqm,
        })
    }

    fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }
}

impl Instance {
    pub fn from_raw(instance: AshInstance) -> Self {
        let inst_arc = Arc::new(UniqueInstance(instance));
        let phys_devices = inst_arc.enumerate_physical_devices().unwrap_or_else(
            |_| Vec::new(),
        );
        let adapters = phys_devices
            .iter()
            .filter_map(|&pd| {
                AdapterData::new(&inst_arc, pd)
                    .ok()
                    .map(RefEqArc::new)
                    .map(Adapter)
            })
            .collect();
        Self {
            instance: inst_arc,
            adapters,
        }
    }
    pub fn instance(&self) -> &AshInstance {
        &self.instance
    }
    pub fn try_take(self) -> Result<AshInstance, Self> {
        match Arc::try_unwrap(self.instance) {
            Ok(i) => Ok(UniqueInstance::take(i)),
            Err(i) => Err(Self {
                instance: i,
                adapters: self.adapters,
            }),
        }
    }
}

impl AsRef<AshInstance> for Instance {
    fn as_ref(&self) -> &AshInstance {
        &self.instance
    }
}

impl core::Instance<ManagedEnvironment> for Instance {
    type Adapter = Adapter;

    fn adapters(&self) -> &[Self::Adapter] {
        &self.adapters
    }
    fn new_device_builder(&self, adapter: &Self::Adapter) -> DeviceBuilder {
        // We need to make sure `adapter` is an element of `self.adapters`
        // or we might have an undefined behavior
        assert!(
            self.adapters.iter().find(|e| e == &adapter).is_some(),
            "the given Adapter does not belong to this"
        );

        unsafe {
            DeviceBuilder::new(
                InstanceRef(self.instance.clone()),
                adapter.0.physical_device,
                adapter.0.eqm,
            )
        }
    }
}

/// `AsRef` wrapper for `Arc<UniqueInstance>` (used internally)
pub struct InstanceRef(Arc<UniqueInstance>);

impl AsRef<AshInstance> for InstanceRef {
    fn as_ref(&self) -> &AshInstance {
        &self.0
    }
}

impl fmt::Debug for InstanceRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // FIXME: write pointer
        f.debug_tuple("InstanceRef").finish()
    }
}

pub type DeviceBuilder = GenericDeviceBuilder<InstanceRef>;

#[derive(Debug, Clone)]
pub struct GenericDeviceBuilder<T: AsRef<AshInstance>> {
    instance: T,
    eqm: EngineQueueMappings,
    physical_device: vk::PhysicalDevice,
    features: vk::PhysicalDeviceFeatures,
    supported_features: vk::PhysicalDeviceFeatures,
    layers: VecDeque<ffi::CString>,
    extensions: HashSet<ffi::CString>,
    supported_extensions: HashSet<ffi::CString>,
}

impl<T: AsRef<AshInstance>> GenericDeviceBuilder<T> {
    /// Constructs a new `DeviceBuilder`.
    ///
    /// - The specified Vulkan instance pointed at by `instance` must be valid
    ///   and outlive the created `DeviceBuilder`.
    /// - The specified `physical_device` must be valid.
    pub unsafe fn new(
        instance: T,
        physical_device: vk::PhysicalDevice,
        eqm: EngineQueueMappings,
    ) -> Self {
        let supported_features = instance.as_ref().get_physical_device_features(
            physical_device,
        );
        let ext_props = instance
            .as_ref()
            .enumerate_device_extension_properties(physical_device)
            .unwrap();
        let mut supported_extensions = HashSet::new();

        for ext in ext_props.iter() {
            let name = ffi::CStr::from_ptr(ext.extension_name.as_ptr());
            supported_extensions.insert(name.to_owned());
        }

        Self {
            instance,
            physical_device,
            eqm,
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
        cname
            .map(|n| self.supported_extensions.contains(&n))
            .unwrap_or(false)
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
    pub fn info(&self) -> (DeviceCreateInfo, EngineQueueMappings, DeviceCapabilities) {
        // Calculate the number of queues required for each queue family
        let mut used_count = HashMap::new();
        let eqm = self.eqm;
        for mapping in eqm.into_array().iter() {
            if !used_count.contains_key(&mapping.queue_family_index) {
                used_count.insert(mapping.queue_family_index, 1);
            } else {
                *used_count.get_mut(&mapping.queue_family_index).unwrap() += 1;
            }
        }

        let mut queue_create_infos = Vec::new();
        for (&fam_idx, &count) in used_count.iter() {
            queue_create_infos.push(DeviceQueueCreateInfo {
                p_next: ptr::null(),
                flags: vk::DeviceQueueCreateFlags::empty(),
                queue_family_index: fam_idx as u32,
                queue_priorities: vec![0.5f32; count as usize],
            });
        }

        let cap =
            DeviceCapabilities::new(self.instance.as_ref(), self.physical_device, &self.features);

        // TODO: enable debug markers (VK_EXT_debug_marker) somehow

        (
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
        )
    }

    /// Constructs a `AshDevice`.
    pub unsafe fn build_raw(
        &self,
    ) -> Result<(ash::Device<V1_0>, EngineQueueMappings, DeviceCapabilities), DeviceBuildError> {
        let (dci, eqm, dc) = self.info();
        let inst = self.instance.as_ref();
        let dci_raw = dci.as_raw();
        let dev = inst.create_device(self.physical_device, &dci_raw, None)
            .map_err(|e| match e {
                ash::DeviceError::LoadError(errors) => DeviceBuildError::LoadError(errors),
                ash::DeviceError::VkError(vk::Result::ErrorInitializationFailed) => {
                    DeviceBuildError::InitializationFailed
                }
                ash::DeviceError::VkError(vk::Result::ErrorExtensionNotPresent) => {
                    DeviceBuildError::ExtensionNotPresent
                }
                ash::DeviceError::VkError(vk::Result::ErrorFeatureNotPresent) => {
                    DeviceBuildError::FeatureNotPresent
                }
                ash::DeviceError::VkError(vk::Result::ErrorTooManyObjects) => {
                    DeviceBuildError::TooManyObjects
                }
                ash::DeviceError::VkError(e) => DeviceBuildError::GenericError(
                    translate_generic_error_unwrap(e),
                ),
            })?;
        Ok((dev, eqm, dc))
    }
}

impl core::DeviceBuilder<ManagedEnvironment> for DeviceBuilder {
    type BuildError = DeviceBuildError;
    fn build(&self) -> Result<Device<ManagedDeviceRef>, Self::BuildError> {
        unsafe {
            self.build_raw().map(|(dev, eqm, dc)| {
                let inst_ref = OwnedInstanceRef { instance: self.instance.0.clone() };
                let dev_ref = ManagedDeviceRef::from_raw(dev, inst_ref);
                Device::new(dev_ref, eqm, dc)
            })
        }
    }
}
