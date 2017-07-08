//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use {RefEqArc, DeviceRef, Backend};

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

pub struct PipelineLayout<T: DeviceRef> {
    data: RefEqArc<PipelineLayoutData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for PipelineLayout<T> => data
}

#[derive(Debug)]
struct PipelineLayoutData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::PipelineLayout for PipelineLayout<T> {}

impl<T: DeviceRef> core::Marker for PipelineLayout<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
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
