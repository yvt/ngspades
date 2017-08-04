//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use std::sync::Mutex;
use std::{mem, ptr};
use ash::vk;
use ash::version::DeviceV1_0;
use smallvec::SmallVec;

use {RefEqArc, DeviceRef, AshDevice, Backend, translate_generic_error_unwrap};
use command::mutex::{ResourceMutex, ResourceMutexDeviceRef};
use imp::LlFence;

pub struct DescriptorSetLayout<T: DeviceRef> {
    data: RefEqArc<DescriptorSetLayoutData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for DescriptorSetLayout<T> => data
}

#[derive(Debug)]
struct DescriptorSetLayoutData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::DescriptorSetLayout for DescriptorSetLayout<T> {}

impl<T: DeviceRef> core::Marker for DescriptorSetLayout<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> DescriptorSetLayout<T> {
    pub fn handle(&self) -> vk::DescriptorSetLayout {
        unimplemented!()
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
    fn set_label(&self, label: Option<&str>) {
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

    pub(crate) fn device_ref(&self) -> &T {
        &self.data.device_ref
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

    fn deallocate(&mut self, allocation: &mut Self::Allocation) {
        unimplemented!()
    }

    fn make_descriptor_set(
        &mut self,
        description: &core::DescriptorSetDescription<DescriptorSetLayout<T>>,
    ) -> core::Result<Option<(DescriptorSet<T>, Self::Allocation)>> {
        unimplemented!()
    }

    fn reset(&mut self) {
        unimplemented!()
    }
}

impl<T: DeviceRef> DescriptorPool<T> {
    pub fn handle(&self) -> vk::DescriptorPool {
        self.data.handle
    }
}

impl<T: DeviceRef> core::Marker for DescriptorPool<T> {
    fn set_label(&self, label: Option<&str>) {
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
    device_ref: T,
    handle: vk::DescriptorSet,
    pool: DescriptorPool<T>,
}

impl<T: DeviceRef> Drop for DescriptorSetLockData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.free_descriptor_sets(self.pool.handle(), &[self.handle]) };
    }
}

impl<T: DeviceRef> core::DescriptorSet<Backend<T>> for DescriptorSet<T> {
    fn update(&self, writes: &[core::WriteDescriptorSet<Backend<T>>]) {
        // TODO: add references to the objects
        let mut locked = self.data.mutex.lock().unwrap();
        let mut lock_data = locked.lock_host_write();
        let device: &AshDevice = lock_data.device_ref.device();
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
        let mut locked = self.data.mutex.lock().unwrap();
        let mut lock_data = locked.lock_host_write();
        let device: &AshDevice = lock_data.device_ref.device();
        unimplemented!()
    }
}

impl<T: DeviceRef> core::Marker for DescriptorSet<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> DescriptorSet<T> {
    pub fn handle(&self) -> vk::DescriptorSet {
        self.data.handle
    }

    pub(crate) fn lock_device(
        &self,
    ) -> ResourceMutexDeviceRef<LlFence<T>, DescriptorSetLockData<T>> {
        self.data.mutex.lock().unwrap().lock_device()
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
