//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use std::fmt;
use ash::vk;
use std::sync::Arc;
use atomic_refcell::AtomicRefCell;

use {DeviceRef, Backend};
use imp::Fence;

/// Used to encode a render subpass with secondary command buffers.
pub(super) struct NestedPassEncoder<T: DeviceRef> {
    secondary_buffers: Vec<Arc<AtomicRefCell<Option<SecondaryCommandBufferData<T>>>>>,
    used_count: usize,
}

impl<T: DeviceRef> fmt::Debug for NestedPassEncoder<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("NestedPassEncoder")
            .field("used_count", &self.used_count)
            .finish()
    }
}

impl<T: DeviceRef> NestedPassEncoder<T> {
    pub fn new() -> Self {
        Self {
            secondary_buffers: Vec::new(),
            used_count: 0,
        }
    }

    pub fn make_secondary_command_buffer<F>(
        &mut self,
        device_ref: &T,
        buffer_allocator: F,
    ) -> SecondaryCommandBuffer<T>
    where
        F: FnOnce() -> vk::CommandBuffer,
    {
        if self.used_count == self.secondary_buffers.len() {
            self.secondary_buffers.push(Arc::new(
                AtomicRefCell::new(Some(SecondaryCommandBufferData {
                    device_ref: device_ref.clone(),

                    buffer: buffer_allocator(),
                    wait_fences: Vec::new(),
                    update_fences: Vec::new(),
                })),
            ));
        }
        self.used_count += 1;
        let ref mut next_sb_data = self.secondary_buffers[self.used_count - 1];
        let sb_data = Arc::get_mut(next_sb_data)
            .unwrap()
            .borrow_mut()
            .take()
            .unwrap();
        SecondaryCommandBuffer { data: Some((sb_data, next_sb_data.clone())) }
    }

    pub fn start(&mut self) {
        assert_eq!(self.used_count, 0);
    }

    pub fn end<F>(&mut self, mut cb: F)
    where
        F: FnMut(&mut SecondaryCommandBufferData<T>) -> (),
    {
        for i in 0..self.used_count {
            let sb_data_arc = &mut self.secondary_buffers[i];
            let sb_data_cell = Arc::get_mut(sb_data_arc).expect(
                "missing call to end_encoding on one of the secondary command buffers",
            );
            // since `AtomicRefCell` does not have `get_mut`, we need to use less performant
            // `borrow_mut`
            let mut sb_data_opt = sb_data_cell.borrow_mut();
            let mut sb_data = sb_data_opt.as_mut().unwrap();
            cb(&mut sb_data);
            sb_data.wait_fences.clear();
            sb_data.update_fences.clear();
        }
        self.used_count = 0;
    }
}

pub struct SecondaryCommandBuffer<T: DeviceRef> {
    data: Option<
        (SecondaryCommandBufferData<T>,
         Arc<AtomicRefCell<Option<SecondaryCommandBufferData<T>>>>),
    >,
}

impl<T: DeviceRef> fmt::Debug for SecondaryCommandBuffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.data {
            &Some((ref work, _)) => {
                f.debug_struct("SecondaryCommandBuffer")
                    .field("data", work)
                    .finish()
            }
            &None => f.debug_struct("SecondaryCommandBuffer").finish(),
        }
    }
}

impl<T: DeviceRef> SecondaryCommandBuffer<T> {
    pub(super) fn exepct_active(&self) -> &SecondaryCommandBufferData<T> {
        &self.data
            .as_ref()
            .expect("this secondary command buffer is not recording")
            .0
    }
    pub(super) fn exepct_active_mut(&mut self) -> &mut SecondaryCommandBufferData<T> {
        &mut self.data
            .as_mut()
            .expect("this secondary command buffer is not recording")
            .0
    }
}

pub(super) struct SecondaryCommandBufferData<T: DeviceRef> {
    pub(super) device_ref: T,

    pub(super) buffer: vk::CommandBuffer,
    pub(super) wait_fences: Vec<(Fence<T>, core::PipelineStageFlags, core::AccessTypeFlags)>,
    pub(super) update_fences: Vec<(Fence<T>, core::PipelineStageFlags, core::AccessTypeFlags)>,
}

impl<T: DeviceRef> fmt::Debug for SecondaryCommandBufferData<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SecondaryCommandBufferData")
            .field("device_ref", &self.device_ref)
            .field("buffer", &self.buffer)
            .field("wait_fences", &self.wait_fences)
            .field("update_fences", &self.update_fences)
            .finish()
    }
}

impl<T: DeviceRef> core::SecondaryCommandBuffer<Backend<T>> for SecondaryCommandBuffer<T> {
    fn end_encoding(&mut self) {
        let (work, result_cell) = self.data.take().expect(
            "this secondary command buffer is not recording",
        );
        let mut result = result_cell.borrow_mut();
        assert!(result.is_none());
        *result = Some(work);
    }
}

impl<T: DeviceRef> core::Marker for SecondaryCommandBuffer<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
