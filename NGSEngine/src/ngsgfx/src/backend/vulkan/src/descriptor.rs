//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use std::{mem, ptr};
use ash::vk;
use ash::version::DeviceV1_0;
use smallvec::SmallVec;

use {RefEqArc, DeviceRef, AshDevice, Backend, translate_generic_error_unwrap};

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
    device: T,
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
    device: T,
}

impl<T: DeviceRef> core::DescriptorSet<Backend<T>> for DescriptorSet<T> {
    fn update(&self, writes: &[core::WriteDescriptorSet<Backend<T>>]) {
        unimplemented!()
    }
    fn copy_from(&self, copies: &[core::CopyDescriptorSet<Self>]) {
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
        unimplemented!()
    }
}
