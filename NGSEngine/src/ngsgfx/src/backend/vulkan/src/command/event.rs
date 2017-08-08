//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use ash::version::DeviceV1_0;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::Arc;
use std::{ptr, mem};
use parking_lot::Mutex;

use {RefEqArc, DeviceRef, AshDevice, translate_generic_error_unwrap};
use imp::{DescriptorSetLockData, DescriptorSet, BufferLockData, Buffer, Framebuffer,
          FramebufferLockData};
use super::mutex::{ResourceFence, ResourceFenceDependencyTable, ResourceMutexDeviceRef};
use super::recycler::Recycler;
use super::buffer::CommandBufferPoolSet;
use super::fence::FenceLockData;

/// Low-level fence. Implements `ResourceFence`.
#[derive(Debug)]
pub(crate) struct LlFence<T: DeviceRef> {
    /// uses `Option` to implement `Drop`
    data: Option<Mutex<LlFenceData<T>>>,
    recycler: Arc<Recycler<LlFenceData<T>>>,

    /// Copy of `LlFenceData::fences`. Do not destroy!
    fences: Vec<vk::Fence>,
}

#[derive(Debug)]
pub(crate) struct LlFenceFactory<T: DeviceRef>(Arc<Recycler<LlFenceData<T>>>, T);

impl<T: DeviceRef> LlFenceFactory<T> {
    pub fn new(device_ref: T) -> Self {
        LlFenceFactory(Arc::new(Recycler::new()), device_ref)
    }

    pub fn build(&self, num_fences: usize, signaled: bool) -> core::Result<LlFence<T>> {
        let data = LlFenceData::new(self.1.clone(), num_fences, signaled)?;
        Ok(LlFence {
            fences: data.fences.clone(),
            data: Some(Mutex::new(data)),
            recycler: self.0.clone(),
        })
    }
}

#[derive(Debug)]
struct LlFenceData<T: DeviceRef> {
    device_ref: T,
    fences: Vec<vk::Fence>,
    state: LlFenceState,

    descriptor_sets: ResourceFenceDependencyTable<LlFence<T>, DescriptorSetLockData<T>>,
    buffers: ResourceFenceDependencyTable<LlFence<T>, BufferLockData<T>>,
    cbp_sets: ResourceFenceDependencyTable<LlFence<T>, CommandBufferPoolSet<T>>,
    semaphores: ResourceFenceDependencyTable<LlFence<T>, FenceLockData<T>>,
    framebuffers: ResourceFenceDependencyTable<LlFence<T>, FramebufferLockData<T>>,
}

#[derive(Debug, PartialEq, Eq)]
enum LlFenceState {
    Initial,
    Unsignaled,
    Signaled,
}

#[derive(Debug)]
pub(super) struct LlFenceDepInjector<'a, T: DeviceRef>(&'a mut LlFenceData<T>, &'a Arc<LlFence<T>>);

impl<T: DeviceRef> ResourceFence for LlFence<T> {
    fn check_fence(&self, wait: bool) {
        self.data.as_ref().unwrap().lock().check_fence(
            wait,
            Some(self),
        )
    }
}

impl<T: DeviceRef> LlFence<T> {
    pub(super) fn inject_deps<F>(this: &Arc<Self>, cb: F)
    where
        F: FnOnce(LlFenceDepInjector<T>),
    {
        let mut data = this.data.as_ref().unwrap().lock();

        assert_eq!(data.state, LlFenceState::Initial);

        cb(LlFenceDepInjector(&mut data, this));
    }

    pub fn mark_submitted(&self) {
        let mut data = self.data.as_ref().unwrap().lock();
        assert_eq!(data.state, LlFenceState::Initial);
        data.state = LlFenceState::Unsignaled;
    }

    /// List containing a `vk::Fence` for each internal queue.
    pub fn fences(&self) -> &[vk::Fence] {
        &self.fences
    }

    fn wait(&self, _: Duration) -> core::Result<bool> {
        unimplemented!()
    }

    fn reset(&self) -> core::Result<()> {
        let mut data = self.data.as_ref().unwrap().lock();
        match data.state {
            LlFenceState::Unsignaled => {
                data.check_fence(false, Some(self));
                if data.state == LlFenceState::Unsignaled {
                    mem::drop(data);
                    panic!("fence is still in use");
                }
            }
            LlFenceState::Initial => {
                return Ok(());
            }
            LlFenceState::Signaled => {}
        }

        {
            let device: &AshDevice = data.device_ref.device();
            unsafe {
                device.reset_fences(data.fences.as_slice()).map_err(
                    translate_generic_error_unwrap,
                )?;
            }
        }
        data.state = LlFenceState::Initial;
        Ok(())
    }
}

impl<T: DeviceRef> Drop for LlFence<T> {
    fn drop(&mut self) {
        let data = self.data.take().unwrap().into_inner();
        if data.state == LlFenceState::Unsignaled {
            // Wait for the completion in a background thread
            self.recycler.recycle(data);
        }
    }
}

impl<'a, T: DeviceRef> LlFenceDepInjector<'a, T> {
    /// Move all dependencies from `source` to `this`.
    pub fn inherit(&mut self, source: &mut CommandDependencyTable<T>) {
        for (_, rmdr) in source.descriptor_sets.drain() {
            self.0.descriptor_sets.insert(self.1, rmdr);
        }
        for (_, rmdr) in source.buffers.drain() {
            self.0.buffers.insert(self.1, rmdr);
        }
        for (_, rmdr) in source.framebuffers.drain() {
            self.0.framebuffers.insert(self.1, rmdr);
        }
    }

    pub fn insert_cbp_set(
        &mut self,
        cbp_set: ResourceMutexDeviceRef<LlFence<T>, CommandBufferPoolSet<T>>,
    ) {
        self.0.cbp_sets.insert(self.1, cbp_set);
    }

    pub fn insert_semaphores(
        &mut self,
        fence_lock_data: ResourceMutexDeviceRef<LlFence<T>, FenceLockData<T>>,
    ) {
        self.0.semaphores.insert(self.1, fence_lock_data);
    }
}

impl<T: DeviceRef> LlFenceData<T> {
    fn new(device_ref: T, num_fences: usize, signaled: bool) -> core::Result<Self> {
        let mut data = Self {
            device_ref,
            fences: Vec::new(),
            state: if signaled {
                LlFenceState::Signaled
            } else {
                LlFenceState::Initial
            },

            descriptor_sets: ResourceFenceDependencyTable::new(),
            buffers: ResourceFenceDependencyTable::new(),
            cbp_sets: ResourceFenceDependencyTable::new(),
            semaphores: ResourceFenceDependencyTable::new(),
            framebuffers: ResourceFenceDependencyTable::new(),
        };

        {
            let device: &AshDevice = data.device_ref.device();
            for _ in 0..num_fences {
                data.fences.push(unsafe {
                    device.create_fence(
                        &vk::FenceCreateInfo {
                            s_type: vk::StructureType::FenceCreateInfo,
                            p_next: ptr::null(),
                            flags: if signaled {
                                vk::FENCE_CREATE_SIGNALED_BIT
                            } else {
                                vk::FenceCreateFlags::empty()
                            },
                        },
                        data.device_ref.allocation_callbacks(),
                    )
                }.map_err(translate_generic_error_unwrap)?);
            }
        }

        Ok(data)
    }

    fn check_fence(&mut self, wait: bool, fence: Option<&LlFence<T>>) {
        if self.state == LlFenceState::Signaled {
            return;
        }

        let device: &AshDevice = self.device_ref.device();
        if wait {
            'a: loop {
                match unsafe { device.wait_for_fences(self.fences.as_slice(), true, 1000000000) } {
                    Ok(()) => {
                        break 'a;
                    }
                    Err(vk::Result::Timeout) => {
                        // Try again...
                    }
                    Err(_) => {
                        // There is nothing we can do other than ignoring this error
                        break 'a;
                    }
                }
            }
        } else {
            match unsafe { device.wait_for_fences(self.fences.as_slice(), true, 0) } {
                Ok(()) => {}
                Err(vk::Result::Timeout) => {
                    // Not signaled
                    return;
                }
                Err(_) => {
                    // There is nothing we can do other than ignoring this error
                }
            }
        }

        self.descriptor_sets.clear(fence);
        self.buffers.clear(fence);
        self.cbp_sets.clear(fence);
        self.semaphores.clear(fence);
        self.framebuffers.clear(fence);
        self.state = LlFenceState::Signaled;
    }
}

impl<T: DeviceRef> Drop for LlFenceData<T> {
    fn drop(&mut self) {
        self.check_fence(true, None);

        let device: &AshDevice = self.device_ref.device();
        for &fence in self.fences.iter() {
            unsafe {
                device.destroy_fence(fence, self.device_ref.allocation_callbacks());
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct CommandDependencyTable<T: DeviceRef> {
    descriptor_sets:
        HashMap<DescriptorSet<T>, ResourceMutexDeviceRef<LlFence<T>, DescriptorSetLockData<T>>>,
    buffers: HashMap<Buffer<T>, ResourceMutexDeviceRef<LlFence<T>, BufferLockData<T>>>,
    framebuffers:
        HashMap<Framebuffer<T>, ResourceMutexDeviceRef<LlFence<T>, FramebufferLockData<T>>>,
    // TODO: graphics pipelines
    // TODO: compute pipelines
    // TODO: stencil states
    // TODO: images
    // TODO: pipeline layouts?
}

impl<T: DeviceRef> CommandDependencyTable<T> {
    pub fn new() -> Self {
        Self {
            descriptor_sets: Default::default(),
            buffers: Default::default(),
            framebuffers: Default::default(),
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.descriptor_sets.clear();
        self.buffers.clear();
        self.framebuffers.clear();
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

    pub fn insert_framebuffer(&mut self, obj: &Framebuffer<T>) {
        if self.framebuffers.contains_key(obj) {
            return;
        }

        let device_ref = obj.lock_device();
        self.framebuffers.insert(obj.clone(), device_ref);
    }

    /// Move all dependencies from `source` to `self`.
    pub fn inherit(&mut self, source: &mut Self) {
        self.descriptor_sets.extend(source.descriptor_sets.drain());
        self.buffers.extend(source.buffers.drain());
        self.framebuffers.extend(source.framebuffers.drain());
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
    /// Must be `Some(_)`
    ll: Arc<LlFence<T>>,
}

impl<T: DeviceRef> Event<T> {
    pub(crate) fn new(ll: Arc<LlFence<T>>) -> Self {
        Self { data: RefEqArc::new(EventData { ll }) }
    }

    pub(super) fn llfence(&self) -> &Arc<LlFence<T>> {
        &self.data.ll
    }
}

impl<T: DeviceRef> core::Event for Event<T> {
    fn reset(&self) -> core::Result<()> {
        self.data.ll.reset()
    }
    fn wait(&self, duration: Duration) -> core::Result<bool> {
        self.data.ll.wait(duration)
    }
}

impl<T: DeviceRef> core::Marker for Event<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}
