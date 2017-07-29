//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use std::{fmt, mem};
use ash::vk;
use ash::version::DeviceV1_0;
use std::sync::Arc;
use atomic_refcell::AtomicRefCell;

use {DeviceRef, Backend, AshDevice, translate_generic_error_unwrap};
use imp::{Fence, CommandDependencyTable};

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

    /// Creates a secondary command buffer that encodes nothing.
    ///
    /// This is sometimes required to handle error states in the command encoder.
    pub fn make_noop_secondary_command_buffer(&mut self) -> SecondaryCommandBuffer<T> {
        SecondaryCommandBuffer { data: SecondaryCommandBufferState::Noop }
    }

    pub fn make_secondary_command_buffer<F>(
        &mut self,
        device_ref: &T,
        buffer_allocator: &mut F,
    ) -> core::Result<SecondaryCommandBuffer<T>>
    where
        F: FnMut() -> core::Result<vk::CommandBuffer>,
    {
        if self.used_count == self.secondary_buffers.len() {
            self.secondary_buffers.push(Arc::new(
                AtomicRefCell::new(Some(SecondaryCommandBufferData {
                    device_ref: device_ref.clone(),

                    buffer: buffer_allocator()?,
                    wait_fences: Vec::new(),
                    update_fences: Vec::new(),
                    dependency_table: CommandDependencyTable::new(),

                    result: Ok(()),
                })),
            ));
        }
        self.used_count += 1;
        let ref mut next_sb_data = self.secondary_buffers[self.used_count - 1];
        let sb_data = match Arc::get_mut(next_sb_data).unwrap().borrow_mut().take() {
            Some(sb_data) => sb_data,
            None => {
                // FIXME: this means someone still has `SecondaryCommandBufferData`
                //        and this is very unsafe!
                SecondaryCommandBufferData {
                    device_ref: device_ref.clone(),

                    buffer: buffer_allocator()?,
                    wait_fences: Vec::new(),
                    update_fences: Vec::new(),
                    dependency_table: CommandDependencyTable::new(),

                    result: Ok(()),
                }
            }
        };

        Ok(SecondaryCommandBuffer {
            data: SecondaryCommandBufferState::Encoding(sb_data, next_sb_data.clone()),
        })
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
    data: SecondaryCommandBufferState<T>,
}

derive_using_field! {
    (T: DeviceRef); (Debug) for SecondaryCommandBuffer<T> => data
}

enum SecondaryCommandBufferState<T: DeviceRef> {
    Encoding(
        SecondaryCommandBufferData<T>,
        Arc<AtomicRefCell<Option<SecondaryCommandBufferData<T>>>>
    ),
    NotEncoding,
    Noop,
}

impl<T: DeviceRef> fmt::Debug for SecondaryCommandBufferState<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &SecondaryCommandBufferState::Encoding(ref work, _) => {
                f.debug_tuple("SecondaryCommandBufferState::Encoding")
                    .field(work)
                    .finish()
            }
            &SecondaryCommandBufferState::NotEncoding => {
                f.debug_tuple("SecondaryCommandBufferState::NotEncoding")
                    .finish()
            }
            &SecondaryCommandBufferState::Noop => {
                f.debug_tuple("SecondaryCommandBufferState::NotEncoding")
                    .finish()
            }
        }
    }
}

impl<T: DeviceRef> SecondaryCommandBuffer<T> {
    pub(super) fn expect_active(&self) -> Option<&SecondaryCommandBufferData<T>> {
        match self.data {
            SecondaryCommandBufferState::Encoding(ref work, _) => Some(work),
            SecondaryCommandBufferState::NotEncoding => {
                panic!("this secondary command buffer is not recording")
            }
            SecondaryCommandBufferState::Noop => None,
        }
    }
    pub(super) fn expect_active_mut(&mut self) -> Option<&mut SecondaryCommandBufferData<T>> {
        match self.data {
            SecondaryCommandBufferState::Encoding(ref mut work, _) => Some(work),
            SecondaryCommandBufferState::NotEncoding => {
                panic!("this secondary command buffer is not recording")
            }
            SecondaryCommandBufferState::Noop => None,
        }
    }

    pub(super) fn release(&mut self) {
        let (work, result_cell) =
            match mem::replace(&mut self.data, SecondaryCommandBufferState::NotEncoding) {
                SecondaryCommandBufferState::Encoding(work, result_cell) => (work, result_cell),
                SecondaryCommandBufferState::NotEncoding => unreachable!(),
                SecondaryCommandBufferState::Noop => return,
            };
        let mut result = result_cell.borrow_mut();
        assert!(result.is_none());
        *result = Some(work);
    }

    pub(super) fn dependency_table(&mut self) -> Option<&mut CommandDependencyTable<T>> {
        self.expect_active_mut().map(|x| &mut x.dependency_table)
    }
}

pub(super) struct SecondaryCommandBufferData<T: DeviceRef> {
    pub(super) device_ref: T,

    pub(super) buffer: vk::CommandBuffer,
    pub(super) wait_fences: Vec<(Fence<T>, core::PipelineStageFlags, core::AccessTypeFlags)>,
    pub(super) update_fences: Vec<(Fence<T>, core::PipelineStageFlags, core::AccessTypeFlags)>,
    pub(super) result: core::Result<()>,

    pub(super) dependency_table: CommandDependencyTable<T>,
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
        if let Some(sbd) = self.expect_active_mut() {
            let end_result = {
                let device: &AshDevice = sbd.device_ref.device();
                let buffer = sbd.buffer;
                unsafe {
                    device.end_command_buffer(buffer).map_err(
                        translate_generic_error_unwrap,
                    )
                }
            };
            sbd.result = end_result;
        }

        self.release();
    }
}

impl<T: DeviceRef> core::Marker for SecondaryCommandBuffer<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
