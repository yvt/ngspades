//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use std::time::Duration;
use std::collections::HashMap;

use {RefEqArc, DeviceRef};
use imp::{self, DescriptorSetLockData, DescriptorSet, BufferLockData, Buffer};
use super::tokenlock::{TokenLock, Token};
use super::mutex::{ResourceFence, GetResourceFenceDependencyTable, ResourceFenceDependencyTable,
                   ResourceMutexRef};

#[derive(Debug)]
pub(crate) struct LlFence<T: DeviceRef> {
    data: TokenLock<LlFenceData<T>>,
}

#[derive(Debug)]
struct LlFenceData<T: DeviceRef> {
    descriptor_sets: ResourceFenceDependencyTable<LlFence<T>, DescriptorSetLockData<T>>,
    buffers: ResourceFenceDependencyTable<LlFence<T>, BufferLockData<T>>,
}

impl<T: DeviceRef> GetResourceFenceDependencyTable<DescriptorSetLockData<T>> for LlFence<T> {
    fn get_dependency_table<'a: 'b, 'b>(
        &'a self,
        token: &'b mut Token,
    ) -> &'b mut ResourceFenceDependencyTable<Self, DescriptorSetLockData<T>> {
        &mut self.data.write(token).unwrap().descriptor_sets
    }
}

impl<T: DeviceRef> GetResourceFenceDependencyTable<BufferLockData<T>> for LlFence<T> {
    fn get_dependency_table<'a: 'b, 'b>(
        &'a self,
        token: &'b mut Token,
    ) -> &'b mut ResourceFenceDependencyTable<Self, BufferLockData<T>> {
        &mut self.data.write(token).unwrap().buffers
    }
}

impl<T: DeviceRef> ResourceFence for LlFence<T> {
    fn check_fence(&self) {
        unimplemented!()
    }
}

#[derive(Debug)]
pub(crate) struct CommandDependencyTable<T: DeviceRef> {
    descriptor_sets:
        HashMap<DescriptorSet<T>, ResourceMutexRef<LlFence<T>, DescriptorSetLockData<T>>>,
    buffers: HashMap<Buffer<T>, ResourceMutexRef<LlFence<T>, BufferLockData<T>>>,
    // TODO: graphics pipelines
    // TODO: compute pipelines
    // TODO: stencil states
    // TODO: framebuffer
    // TODO: images
}

impl<T: DeviceRef> CommandDependencyTable<T> {
    pub fn new() -> Self {
        Self {
            descriptor_sets: Default::default(),
            buffers: Default::default(),
        }
    }

    pub fn clear(&mut self) {
        self.descriptor_sets.clear();
        self.buffers.clear();
    }

    pub fn insert_descriptor_set(&mut self, obj: &DescriptorSet<T>) {
        if self.descriptor_sets.contains_key(obj) {
            return;
        }

        let device_ref = obj.lock_device();
        self.descriptor_sets.insert(obj.clone(), device_ref);
    }

    pub fn insert_buffer(&mut self, obj: &Buffer<T>) {
        if self.buffers.contains_key(obj) {
            return;
        }

        let device_ref = obj.lock_device();
        self.buffers.insert(obj.clone(), device_ref);
    }

    /// Move all dependencies from `source` to `self`.
    pub fn inherit(&mut self, source: &mut Self) {
        self.descriptor_sets.extend(source.descriptor_sets.drain());
    }
}

pub struct Event<T: DeviceRef> {
    data: RefEqArc<EventData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Event<T> => data
}

#[derive(Debug)]
struct EventData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::Event for Event<T> {
    fn reset(&self) -> core::Result<()> {
        unimplemented!()
    }
    fn wait(&self, _: Duration) -> core::Result<bool> {
        unimplemented!()
    }
}

impl<T: DeviceRef> core::Marker for Event<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
