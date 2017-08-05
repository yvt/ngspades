//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::{self, vk};
use ash::version::{V1_0, InstanceV1_0, EntryV1_0};
use std::sync::Arc;
use std::{ffi, ptr, fmt};
use std::collections::{VecDeque, HashSet, HashMap};

use imp::{ManagedEnvironment, Device, EngineQueueMappings, DeviceCapabilities, DeviceConfig,
          StorageModeMappings, HeapStrategy, DebugReportConduit};
use ll::{DeviceCreateInfo, DeviceQueueCreateInfo, ApplicationInfo};
use {translate_generic_error_unwrap, RefEqArc, OwnedInstanceRef, AshInstance, InstanceRef, ManagedDeviceRef};

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
    DepthClamp,
    FillModeNonSolid,
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
    debug_report_handlers: Vec<(core::DebugReportTypeFlags, Arc<core::DebugReportHandler>)>,
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
            debug_report_handlers: Vec::new(),
        })
    }

    fn enable_debug_report<T: core::DebugReportHandler + 'static>(&mut self, flags: core::DebugReportTypeFlags, handler: T) {
        if self.enable_extension("VK_EXT_debug_report") {
            self.debug_report_handlers.push((flags, Arc::new(handler)));
        }

    }
    fn enable_validation(&mut self) {
        self.push_back_standard_validation_layer();
    }
    fn enable_debug_marker(&mut self) {
        // TODO: enable `VK_EXT_debug_marker`
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

        let instance_ref = unsafe { OwnedInstanceRef::from_raw(inst) };
        let mut drc = if self.debug_report_handlers.len() > 0 {
            DebugReportConduit::new(&self.entry, &instance_ref).ok()
        } else {
            None
        };

        for drh in self.debug_report_handlers.iter() {
            drc.as_mut().unwrap().add_handler(drh.0, drh.1.clone());
        }

        let mut ngs_instance = unsafe {
            Instance::from_raw(self.entry.clone(), instance_ref)
        };
        ngs_instance.debug_report_conduit = drc.map(Arc::new);
        Ok(ngs_instance)
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
                if self.layers.iter().any(|n| *n == cname) {
                    return true;
                }
                self.layers.push_back(cname);
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
                if self.layers.iter().any(|n| *n == cname) {
                    return true;
                }
                self.layers.push_front(cname);
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

pub struct Instance {
    entry: ash::Entry<V1_0>,
    instance_ref: OwnedInstanceRef,
    adapters: Vec<Adapter>,
    debug_report_conduit: Option<Arc<DebugReportConduit<OwnedInstanceRef>>>,
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
    config: DeviceConfig,
}

impl Adapter {
    /// Return the default `DeviceConfig` for this adapter.
    pub fn device_config(&self) -> &DeviceConfig {
        &self.0.config
    }

    /// Return `vk::PhysicalDevice` for this adapter.
    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.0.physical_device
    }
}

impl core::Adapter for Adapter {
    fn name(&self) -> &str {
        &self.0.name
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
        let mut internal_queues = Vec::new();

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

        let universal_internal_index = 0;
        internal_queues.push((universal_fam_index as u32, universal_index));

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
        let compute_internal_index = if let Some(i) = compute_fam_index {
            used_count[i] += 1;

            internal_queues.push((i as u32, used_count[i] - 1));
            internal_queues.len() - 1
        } else {
            universal_internal_index
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
        let copy_internal_index = if let Some(i) = copy_fam_index {
            used_count[i] += 1;

            internal_queues.push((i as u32, used_count[i] - 1));
            internal_queues.len() - 1
        } else {
            universal_internal_index
        };

        let eqm = EngineQueueMappings {
            universal: universal_internal_index,
            compute: compute_internal_index,
            copy: copy_internal_index,
        };

        let m_props = instance.get_physical_device_memory_properties(physical_device);

        let memory_types: Vec<_> = m_props.memory_types[0..m_props.memory_type_count as usize]
            .iter()
            .map(Clone::clone)
            .collect();

        let smm = {
            let make_sm_map = |flags_list: &[vk::MemoryPropertyFlags]| {
                let mut v = Vec::new();
                for &flags in flags_list.iter() {
                    for (i, memory_type) in memory_types.iter().enumerate() {
                        if memory_type.property_flags.subset(flags) {
                            v.push(i as u8);
                        }
                    }
                }
                v
            };
            StorageModeMappings {
                private: make_sm_map(
                    &[
                        vk::MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
                        vk::MemoryPropertyFlags::empty(),
                    ],
                ),
                shared: make_sm_map(
                    &[
                        vk::MEMORY_PROPERTY_HOST_VISIBLE_BIT |
                            vk::MEMORY_PROPERTY_HOST_COHERENT_BIT,
                    ],
                ),
                memoryless: make_sm_map(
                    &[
                        vk::MEMORY_PROPERTY_DEVICE_LOCAL_BIT |
                            vk::MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT,
                        vk::MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT,
                    ],
                ),
            }
        };

        let memory_type_infos = memory_types
            .iter()
            .map(|memory_type| {
                let ref heap = m_props.memory_heaps[memory_type.heap_index as usize];
                (
                    memory_type.clone(),
                    HeapStrategy::default_with_heap_size(heap.size),
                )
            })
            .collect();

        let config = DeviceConfig {
            queues: internal_queues,
            engine_queue_mappings: eqm,
            memory_types: memory_type_infos,
            storage_mode_mappings: smm,
        };

        Ok(Self {
            physical_device,
            name,
            config,
        })
    }
}

impl Instance {
    pub unsafe fn from_raw(entry: ash::Entry<V1_0>, instance_ref: OwnedInstanceRef) -> Self {
        let phys_devices = instance_ref.instance().enumerate_physical_devices().unwrap_or_else(
            |_| Vec::new(),
        );
        let adapters = phys_devices
            .iter()
            .filter_map(|&pd| {
                AdapterData::new(instance_ref.instance(), pd)
                    .ok()
                    .map(RefEqArc::new)
                    .map(Adapter)
            })
            .collect();
        Self {
            entry,
            instance_ref,
            adapters,
            debug_report_conduit: None,
        }
    }
    pub fn entry(&self) -> &ash::Entry<V1_0> {
        &self.entry
    }
    pub fn instance_ref(&self) -> &OwnedInstanceRef {
        &self.instance_ref
    }
    pub fn instance(&self) -> &AshInstance {
        self.instance_ref.instance()
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

        let mut db = unsafe {
            DeviceBuilder::new(
                self.instance_ref.clone(),
                adapter.0.physical_device,
                adapter.0.config.clone(),
            )
        };
        db.debug_report_conduit = self.debug_report_conduit.clone();
        db
    }
}

pub type DeviceBuilder = GenericDeviceBuilder<OwnedInstanceRef>;

#[derive(Debug, Clone)]
pub struct GenericDeviceBuilder<T: InstanceRef> {
    instance_ref: T,
    config: DeviceConfig,
    physical_device: vk::PhysicalDevice,
    features: vk::PhysicalDeviceFeatures,
    supported_features: vk::PhysicalDeviceFeatures,
    layers: VecDeque<ffi::CString>,
    extensions: HashSet<ffi::CString>,
    supported_extensions: HashSet<ffi::CString>,
    debug_report_conduit: Option<Arc<DebugReportConduit<T>>>,
}

impl<T: InstanceRef> GenericDeviceBuilder<T> {
    /// Constructs a new `DeviceBuilder`.
    ///
    /// - The specified Vulkan instance pointed at by `instance` must be valid
    ///   and outlive the created `DeviceBuilder`.
    /// - The specified `physical_device` must be valid.
    pub unsafe fn new(
        instance_ref: T,
        physical_device: vk::PhysicalDevice,
        config: DeviceConfig,
    ) -> Self {
        let supported_features = instance_ref.instance().get_physical_device_features(
            physical_device,
        );
        let ext_props = instance_ref
            .instance()
            .enumerate_device_extension_properties(physical_device)
            .unwrap();
        let mut supported_extensions = HashSet::new();

        for ext in ext_props.iter() {
            let name = ffi::CStr::from_ptr(ext.extension_name.as_ptr());
            supported_extensions.insert(name.to_owned());
        }

        Self {
            instance_ref,
            physical_device,
            config,
            features: Default::default(),
            supported_features,
            layers: VecDeque::new(),
            extensions: HashSet::new(),
            supported_extensions,
            debug_report_conduit: None,
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
            DeviceFeature::DepthClamp => {
                self.features.depth_clamp = self.supported_features.depth_clamp;
                self.features.depth_clamp != vk::VK_FALSE
            }
            DeviceFeature::FillModeNonSolid => {
                self.features.fill_mode_non_solid = self.supported_features.fill_mode_non_solid;
                self.features.fill_mode_non_solid != vk::VK_FALSE
            }
        }
    }

    /// Attempts to enable all features.
    pub fn enable_all_features(&mut self) -> &mut Self {
        self.enable_feature(DeviceFeature::DepthBounds);
        self.enable_feature(DeviceFeature::CubeArrayImage);
        self.enable_feature(DeviceFeature::DepthClamp);
        self.enable_feature(DeviceFeature::FillModeNonSolid);
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
    pub fn info(&self) -> (DeviceCreateInfo, DeviceConfig, DeviceCapabilities) {
        // Calculate the number of queues required for each queue family
        let mut max_indices = HashMap::new();
        let ref config = self.config;
        for &(family, queue) in config.queues.iter() {
            if let Some(max_index) = max_indices.get_mut(&family) {
                *max_index = ::std::cmp::max(*max_index, queue);
                continue;
            }
            max_indices.insert(family, queue);
        }

        let mut queue_create_infos = Vec::new();
        for (&fam_idx, &max_index) in max_indices.iter() {
            queue_create_infos.push(DeviceQueueCreateInfo {
                p_next: ptr::null(),
                flags: vk::DeviceQueueCreateFlags::empty(),
                queue_family_index: fam_idx as u32,
                queue_priorities: vec![0.5f32; (max_index + 1) as usize],
            });
        }

        let cap =
            DeviceCapabilities::new(self.instance_ref.instance(), self.physical_device, &self.features);

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
            config.clone(),
            cap,
        )
    }

    /// Constructs a `AshDevice`.
    pub unsafe fn build_raw(
        &self,
    ) -> Result<(ash::Device<V1_0>, DeviceConfig, DeviceCapabilities), DeviceBuildError> {
        let (dci, cfg, dc) = self.info();
        let inst = self.instance_ref.instance();
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
        Ok((dev, cfg, dc))
    }
}

impl core::DeviceBuilder<ManagedEnvironment> for DeviceBuilder {
    type BuildError = DeviceBuildError;
    fn build(&self) -> Result<Device<ManagedDeviceRef>, Self::BuildError> {
        unsafe {
            let (dev, cfg, dc) = self.build_raw()?;
            let dev_ref = ManagedDeviceRef::from_raw(dev, (self.instance_ref.clone(), self.debug_report_conduit.clone()));
            Ok(Device::new(dev_ref, cfg, dc))
        }
    }
}
