//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use parking_lot::Mutex;
use std::ptr;
use ash::vk;
use ash::version::DeviceV1_0;
use smallvec::SmallVec;

use {RefEqArc, DeviceRef, AshDevice, Backend, translate_generic_error_unwrap,
     translate_shader_stage_flags};
use command::mutex::{ResourceMutex, ResourceMutexDeviceRef};
use imp::{LlFence, Sampler};

pub struct DescriptorSetLayout<T: DeviceRef> {
    data: RefEqArc<DescriptorSetLayoutData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for DescriptorSetLayout<T> => data
}

#[derive(Debug)]
struct DescriptorSetLayoutData<T: DeviceRef> {
    device_ref: T,
    handle: vk::DescriptorSetLayout,
    imm_samplers: Vec<Sampler<T>>,
}

impl<T: DeviceRef> core::DescriptorSetLayout for DescriptorSetLayout<T> {}

impl<T: DeviceRef> core::Marker for DescriptorSetLayout<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> Drop for DescriptorSetLayoutData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe {
            device.destroy_descriptor_set_layout(self.handle, self.device_ref.allocation_callbacks())
        };
    }
}

impl<T: DeviceRef> DescriptorSetLayout<T> {
    pub(crate) fn new(
        device_ref: &T,
        desc: &core::DescriptorSetLayoutDescription<Sampler<T>>,
    ) -> core::Result<Self> {
        let mut vk_bindings = Vec::with_capacity(desc.bindings.len());
        let mut imm_sampler_is = Vec::with_capacity(desc.bindings.len());
        let mut imm_samplers = Vec::new();
        for binding in desc.bindings.iter() {
            vk_bindings.push(vk::DescriptorSetLayoutBinding {
                binding: binding.location as u32,
                descriptor_type: translate_descriptor_type(binding.descriptor_type),
                descriptor_count: binding.num_elements as u32,
                stage_flags: translate_shader_stage_flags(binding.stage_flags),
                p_immutable_samplers: ptr::null(),
            });
            if let Some(samplers) = binding.immutable_samplers {
                let start_index = imm_samplers.len();
                imm_samplers.extend(samplers.iter().map(|s| (*s).clone()));
                imm_sampler_is.push(Some(start_index..imm_samplers.len()));
            } else {
                imm_sampler_is.push(None);
            }
        }

        let vk_imm_samplers: Vec<_> = imm_samplers.iter().map(|s| s.handle()).collect();
        for (vk_binding, imm_sampler_i) in vk_bindings.iter_mut().zip(imm_sampler_is.iter()) {
            if let &Some(ref imm_sampler_i) = imm_sampler_i {
                vk_binding.p_immutable_samplers = &vk_imm_samplers[imm_sampler_i.start];
            }
        }

        let info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DescriptorSetLayoutCreateInfo,
            p_next: ptr::null(),
            flags: vk::DescriptorSetLayoutCreateFlags::empty(),
            binding_count: vk_bindings.len() as u32,
            p_bindings: vk_bindings.as_ptr(),
        };

        let device_ref = device_ref.clone();
        let handle;
        {
            let device: &AshDevice = device_ref.device();
            handle = unsafe {
                device.create_descriptor_set_layout(&info, device_ref.allocation_callbacks())
            }.map_err(translate_generic_error_unwrap)?;
        }

        Ok(Self {
            data: RefEqArc::new(DescriptorSetLayoutData {
                device_ref,
                handle,
                imm_samplers,
            }),
        })
    }
    pub fn handle(&self) -> vk::DescriptorSetLayout {
        self.data.handle
    }
}

pub struct PipelineLayout<T: DeviceRef> {
    data: RefEqArc<PipelineLayoutData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for PipelineLayout<T> => data
}

#[derive(Debug)]
struct PipelineLayoutData<T: DeviceRef> {
    device_ref: T,
    handle: vk::PipelineLayout,
}

impl<T: DeviceRef> core::PipelineLayout for PipelineLayout<T> {}

impl<T: DeviceRef> core::Marker for PipelineLayout<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> Drop for PipelineLayoutData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe {
            device.destroy_pipeline_layout(self.handle, self.device_ref.allocation_callbacks())
        };
    }
}

impl<T: DeviceRef> PipelineLayout<T> {
    pub(crate) fn new(
        device_ref: &T,
        desc: &core::PipelineLayoutDescription<DescriptorSetLayout<T>>,
    ) -> core::Result<Self> {
        // Four is the upper limit of the number of descriptor sets on
        // some AMD GCN architectures.
        let set_layouts: SmallVec<[_; 4]> = desc.descriptor_set_layouts
            .iter()
            .map(|dsl| dsl.handle())
            .collect();

        let info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PipelineLayoutCreateInfo,
            p_next: ptr::null(),
            flags: vk::PipelineLayoutCreateFlags::empty(), // reserved for future use
            set_layout_count: set_layouts.len() as u32,
            p_set_layouts: set_layouts.as_ptr(),
            push_constant_range_count: 0,
            p_push_constant_ranges: ptr::null(),
        };

        let device_ref = device_ref.clone();
        let handle;
        {
            let device: &AshDevice = device_ref.device();
            handle = unsafe {
                device.create_pipeline_layout(&info, device_ref.allocation_callbacks())
            }.map_err(translate_generic_error_unwrap)?;
        }

        Ok(Self {
            data: RefEqArc::new(PipelineLayoutData { device_ref, handle }),
        })
    }

    pub fn handle(&self) -> vk::PipelineLayout {
        self.data.handle
    }
}

pub struct DescriptorPool<T: DeviceRef> {
    data: RefEqArc<DescriptorPoolData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for DescriptorPool<T> => data
}

#[derive(Debug)]
struct DescriptorPoolData<T: DeviceRef> {
    device_ref: T,
    handle: vk::DescriptorPool,
}

impl<T: DeviceRef> Drop for DescriptorPoolData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe {
            device.destroy_descriptor_pool(self.handle, self.device_ref.allocation_callbacks())
        };
    }
}

impl<T: DeviceRef> core::DescriptorPool<Backend<T>> for DescriptorPool<T> {
    type Allocation = ();

    fn deallocate(&mut self, _: &mut Self::Allocation) {
        // In the Vulkan backend, allocations of descriptor sets are
        // managed by ref-counts completely (for now)
        // We cannot free descriptor sets which are potentially still in use
        // by the device.
    }

    fn make_descriptor_set(
        &mut self,
        description: &core::DescriptorSetDescription<DescriptorSetLayout<T>>,
    ) -> core::Result<Option<(DescriptorSet<T>, Self::Allocation)>> {
        DescriptorSet::new(self, description).map(|r| r.map(|ds| (ds, ())))
    }

    fn reset(&mut self) {
        // No-op (for now)
    }
}

impl<T: DeviceRef> DescriptorPool<T> {
    pub(crate) fn new(
        device_ref: &T,
        desc: &core::DescriptorPoolDescription,
    ) -> core::Result<Self> {
        let vk_pool_sizes: Vec<_> = desc.pool_sizes
            .iter()
            .map(|pool_size| {
                vk::DescriptorPoolSize {
                    typ: translate_descriptor_type(pool_size.descriptor_type),
                    descriptor_count: pool_size.num_descriptors as u32,
                }
            })
            .collect();

        let info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DescriptorPoolCreateInfo,
            p_next: ptr::null(),
            flags: vk::DESCRIPTOR_POOL_CREATE_FREE_DESCRIPTOR_SET_BIT,
            max_sets: desc.max_num_sets as u32,
            pool_size_count: vk_pool_sizes.len() as u32,
            p_pool_sizes: vk_pool_sizes.as_ptr(),
        };

        // TODO: more coarse descriptor set allocation (i.e. respect `DescriptorPoolDescription::supports_deallocation`)

        let device_ref = device_ref.clone();
        let handle;
        {
            let device: &AshDevice = device_ref.device();
            handle = unsafe {
                device.create_descriptor_pool(&info, device_ref.allocation_callbacks())
            }.map_err(translate_generic_error_unwrap)?;
        }

        Ok(Self {
            data: RefEqArc::new(DescriptorPoolData { device_ref, handle }),
        })
    }
    pub fn handle(&self) -> vk::DescriptorPool {
        self.data.handle
    }
}

impl<T: DeviceRef> core::Marker for DescriptorPool<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}

pub struct DescriptorSet<T: DeviceRef> {
    data: RefEqArc<DescriptorSetData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for DescriptorSet<T> => data
}

#[derive(Debug)]
struct DescriptorSetData<T: DeviceRef> {
    /// Copy of `DescriptorSetLockData::handle`. (Do not destroy!)
    handle: vk::DescriptorSet,
    mutex: Mutex<ResourceMutex<LlFence<T>, DescriptorSetLockData<T>>>,
}

#[derive(Debug)]
pub(crate) struct DescriptorSetLockData<T: DeviceRef> {
    handle: vk::DescriptorSet,
    pool: DescriptorPool<T>,
}

impl<T: DeviceRef> Drop for DescriptorSetLockData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.pool.data.device_ref.device();
        unsafe { device.free_descriptor_sets(self.pool.handle(), &[self.handle]) };
    }
}

// TODO: make sure all descriptors filled before the first use?

impl<T: DeviceRef> core::DescriptorSet<Backend<T>> for DescriptorSet<T> {
    fn update(&self, writes: &[core::WriteDescriptorSet<Backend<T>>]) {
        // TODO: add references to the objects
        let mut locked = self.data.mutex.lock();
        let lock_data = locked.lock_host_write();
        let device: &AshDevice = lock_data.pool.data.device_ref.device();
        let mut image_infos: SmallVec<[_; 32]> = SmallVec::new();
        let mut buffer_infos: SmallVec<[_; 32]> = SmallVec::new();
        let write_tmp: SmallVec<[_; 32]> = writes
            .iter()
            .map(|wds| {
                use core::WriteDescriptors::*;
                use translate_image_layout;
                let indices = (image_infos.len(), buffer_infos.len());
                match wds.elements {
                    StorageImage(images) |
                    SampledImage(images) |
                    InputAttachment(images) => {
                        image_infos.extend(images.iter().map(|di| {
                            vk::DescriptorImageInfo {
                                sampler: vk::Sampler::null(),
                                image_view: di.image_view.handle(),
                                image_layout: translate_image_layout(di.image_layout),
                            }
                        }));
                    }
                    Sampler(samplers) => {
                        image_infos.extend(samplers.iter().map(|s| {
                            vk::DescriptorImageInfo {
                                sampler: s.handle(),
                                image_view: vk::ImageView::null(),
                                image_layout: vk::ImageLayout::Undefined,
                            }
                        }));
                    }
                    CombinedImageSampler(iss) => {
                        image_infos.extend(iss.iter().map(|&(ref di, ref s)| {
                            vk::DescriptorImageInfo {
                                sampler: s.handle(),
                                image_view: di.image_view.handle(),
                                image_layout: translate_image_layout(di.image_layout),
                            }
                        }));
                    }
                    ConstantBuffer(dbs) |
                    StorageBuffer(dbs) |
                    DynamicConstantBuffer(dbs) |
                    DynamicStorageBuffer(dbs) => {
                        buffer_infos.extend(dbs.iter().map(|db| {
                            vk::DescriptorBufferInfo {
                                buffer: db.buffer.handle(),
                                offset: db.offset,
                                range: db.range,
                            }
                        }));
                    }
                }
                indices
            })
            .collect();
        let write_tmp: SmallVec<[_; 32]> = writes
            .iter()
            .zip(write_tmp.iter())
            .map(|(wds, &(image_index, buffer_index))| {
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WriteDescriptorSet,
                    p_next: ptr::null(),
                    dst_set: lock_data.handle,
                    dst_binding: wds.start_binding as u32,
                    dst_array_element: wds.start_index as u32,
                    descriptor_count: wds.elements.len() as u32,
                    descriptor_type: translate_descriptor_type(wds.elements.descriptor_type()),
                    p_image_info: image_infos[image_index..].as_ptr(),
                    p_buffer_info: buffer_infos[buffer_index..].as_ptr(),
                    p_texel_buffer_view: ptr::null(),
                }
            })
            .collect();
        unsafe {
            device.update_descriptor_sets(&write_tmp, &[]);
        }
    }

    fn copy_from(&self, copies: &[core::CopyDescriptorSet<Self>]) {
        let _ = copies;
        // let mut locked = self.data.mutex.lock().unwrap();
        // let mut lock_data = locked.lock_host_write();
        // let device: &AshDevice = lock_data.pool.data.device_ref.device();
        unimplemented!()
    }
}

impl<T: DeviceRef> core::Marker for DescriptorSet<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> DescriptorSet<T> {
    /// Host access to `pool` must be externally synchronized
    fn new(
        pool: &DescriptorPool<T>,
        description: &core::DescriptorSetDescription<DescriptorSetLayout<T>>,
    ) -> core::Result<Option<DescriptorSet<T>>> {
        let vk_layouts = [description.layout.handle()];
        let info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DescriptorSetAllocateInfo,
            p_next: ptr::null(),
            descriptor_pool: pool.handle(),
            descriptor_set_count: 1,
            p_set_layouts: vk_layouts.as_ptr(),
        };

        let device: &AshDevice = pool.data.device_ref.device();

        // Vulkan 1.0.55 Specification 13.2. "Descriptor Sets"
        // > Any returned error other than `VK_ERROR_OUT_OF_POOL_MEMORY_KHR` or
        // > `VK_ERROR_FRAGMENTED_POOL` does not imply its usual meaning;
        // > applications should assume that the allocation failed due to
        // > fragmentation, and create a new descriptor pool.
        Ok(unsafe { device.allocate_descriptor_sets(&info) }.ok().map(
            |handles| {
                let handle = handles[0];
                let dsld = DescriptorSetLockData {
                    handle,
                    pool: pool.clone(),
                };
                let dsd = DescriptorSetData {
                    handle,
                    mutex: Mutex::new(ResourceMutex::new(dsld, true)),
                };
                Self { data: RefEqArc::new(dsd) }
            },
        ))
    }

    pub fn handle(&self) -> vk::DescriptorSet {
        self.data.handle
    }

    pub(crate) fn lock_device(
        &self,
    ) -> ResourceMutexDeviceRef<LlFence<T>, DescriptorSetLockData<T>> {
        self.data.mutex.lock().lock_device()
    }
}

fn translate_descriptor_type(value: core::DescriptorType) -> vk::DescriptorType {
    match value {
        core::DescriptorType::StorageImage => vk::DescriptorType::StorageImage,
        core::DescriptorType::SampledImage => vk::DescriptorType::SampledImage,
        core::DescriptorType::Sampler => vk::DescriptorType::Sampler,
        core::DescriptorType::CombinedImageSampler => vk::DescriptorType::CombinedImageSampler,
        core::DescriptorType::ConstantBuffer => vk::DescriptorType::UniformBuffer,
        core::DescriptorType::StorageBuffer => vk::DescriptorType::StorageBuffer,
        core::DescriptorType::DynamicConstantBuffer => vk::DescriptorType::UniformBufferDynamic,
        core::DescriptorType::DynamicStorageBuffer => vk::DescriptorType::StorageBufferDynamic,
        core::DescriptorType::InputAttachment => vk::DescriptorType::InputAttachment,
    }
}
