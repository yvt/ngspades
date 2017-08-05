//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use std::ptr;
use ash::vk;
use ash::version::DeviceV1_0;
use ngsgfx_common::int::BinaryInteger;

use {RefEqArc, DeviceRef, AshDevice, translate_generic_error_unwrap};
use imp;
use super::tokenlock::{TokenLock, Token};
use super::mutex::ResourceMutex;
use super::event::LlFence;

pub struct Fence<T: DeviceRef> {
    data: RefEqArc<FenceData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Fence<T> => data
}

#[derive(Debug)]
struct FenceData<T: DeviceRef> {
    device_ref: T,
    q_data: TokenLock<FenceQueueData<T>>,
    semaphores: Vec<Option<vk::Semaphore>>,
    num_iqs: usize,

    /// A set of internal queues allowed to wait on this fence.
    wait_iq_flags: u32,
}

impl<T: DeviceRef> core::Fence for Fence<T> {}

impl<T: DeviceRef> core::Marker for Fence<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}

/// References to `vk::Semaphore`s protected by `ResourceMutex`
#[derive(Debug)]
pub(super) struct FenceLockData<T: DeviceRef> {
    device_ref: T,
    semaphores: Vec<Option<vk::Semaphore>>,
}

impl<T: DeviceRef> Drop for FenceLockData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        for sem in self.semaphores.iter() {
            if let &Some(sem) = sem {
                unsafe {
                    device.destroy_semaphore(sem, self.device_ref.allocation_callbacks());
                }
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct FenceQueueData<T: DeviceRef> {
    /// `FenceSemState` for each internal queue + external consumers.
    pub wait_states: Vec<FenceWaitState>,

    /// Immutable object that holds references to `vk::Semaphore`s
    pub mutex: ResourceMutex<LlFence<T>, FenceLockData<T>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum FenceWaitState {
    /// The corresponding internal queue is not allowed to wait on this fence.
    Unavailable,

    /// Should wait for this fence by `vkCmdPipelineBarrier`.
    /// (Copy queues do not support events)
    PipelineBarrier {
        /// Specifies the pipeline stages and access types to include in the
        /// first synchronization/access scope.
        src_scope: (core::PipelineStageFlags, core::AccessTypeFlags),
        /// The pipeline stages and access types that were already included
        /// in the second synchronization/access scope. (For example, a render
        /// pass might define external dependencies)
        dst_scope: (core::PipelineStageFlags, core::AccessTypeFlags),
    },

    /// Should wait for this fence by using this semaphore.
    ///
    /// The semaphore to wait is the corresponding element of
    /// `FenceData::semaphores`.
    Semaphore {
        /// Indicates the internal queue index by which the semaphore
        /// was/will be signalled.
        signalled_by: u32,
    },

    /// Wait operation is no-op.
    ///
    ///  - The fence has never been updated.
    ///  - Or, the semaphore was waited on by this internal queue.
    ///    Note: There must be a corresponding signal operation for every
    ///    signal wait operation, which means we cannot wait on the same
    ///    semaphore again with a different stage mask.
    Ready,
}

impl<T: DeviceRef> Fence<T> {
    /// Unstable API: Construct a `Fence` to be used with the command queue `queue`.
    ///
    /// The created `Fence` only can be waited on by the specified consumers.
    /// Each set bit of `dest_queue_flags` specifies an internal queue that is
    /// allowed to wait on this fence. Additionally, `num_dest_externals`
    /// semaphores that can be waited on by external entities are provided.
    ///
    /// TODO: more generic external semahpores
    pub fn new(
        queue: &imp::CommandQueue<T>,
        dest_queue_flags: u32,
        num_dest_externals: usize,
    ) -> core::Result<Self> {
        let num_iqs = queue.device_config().queues.len();
        let wait_states: Vec<_> = (0..(num_iqs + num_dest_externals))
            .map(|i| if i >= num_iqs || dest_queue_flags.get_bit(i as u32) {
                FenceWaitState::Ready
            } else {
                FenceWaitState::Unavailable
            })
            .collect();

        let mut l_data = FenceLockData {
            device_ref: queue.device_ref().clone(),
            semaphores: vec![None; wait_states.len()],
        };

        // Create semaphores in a separate step for proper error handling
        {
            let device: &AshDevice = l_data.device_ref.device();
            for (i, ws) in wait_states.iter().enumerate() {
                if let &FenceWaitState::Ready = ws {
                    l_data.semaphores[i] = unsafe {
                        device.create_semaphore(
                            &vk::SemaphoreCreateInfo {
                                s_type: vk::StructureType::SemaphoreCreateInfo,
                                p_next: ptr::null(),
                                flags: vk::SemaphoreCreateFlags::empty(),
                            },
                            l_data.device_ref.allocation_callbacks(),
                        )
                    }.map_err(translate_generic_error_unwrap)
                        .map(Some)?;
                }
            }
        }
        let semaphores = l_data.semaphores.clone();
        let q_data = FenceQueueData {
            wait_states,
            mutex: ResourceMutex::new(l_data, false),
        };
        let data = FenceData {
            device_ref: queue.device_ref().clone(),
            semaphores,
            q_data: TokenLock::new(queue.token_ref().clone(), q_data),
            num_iqs,
            wait_iq_flags: dest_queue_flags,
        };

        Ok(Self { data: RefEqArc::new(data) })
    }

    pub(super) fn with_description(
        queue: &imp::CommandQueue<T>,
        desc: &core::FenceDescription,
    ) -> core::Result<Self> {
        let _ = (queue, desc);
        unimplemented!()
    }

    pub(super) fn get_semaphore(&self, dest_iq: usize) -> vk::Semaphore {
        self.data.semaphores[dest_iq].unwrap()
    }

    // TODO: retrieve semaphores for external entities

    pub(super) fn queue_data_write<'a: 'b, 'b>(
        &'a self,
        token: &'b mut Token,
    ) -> &'b mut FenceQueueData<T> {
        self.data.q_data.write(token).unwrap()
    }

    pub(super) fn expect_waitable_by_iq(&self, iq: usize) {
        assert!(
            self.data.wait_iq_flags.get_bit(iq as u32),
            "This fence is not configured to be waited for by the specified engine"
        );
    }
}
