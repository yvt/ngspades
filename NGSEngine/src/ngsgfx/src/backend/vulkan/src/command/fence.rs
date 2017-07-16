//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::vk;
use std::time::Duration;

use {RefEqArc, DeviceRef};
use imp;
use super::tokenlock::TokenLock;

pub struct Fence<T: DeviceRef> {
    data: RefEqArc<FenceData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Fence<T> => data
}

#[derive(Debug)]
struct FenceData<T: DeviceRef> {
    device: T,
    q_data: TokenLock<FenceQueueData>,
}

impl<T: DeviceRef> core::Fence for Fence<T> {}

impl<T: DeviceRef> core::Marker for Fence<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}

#[derive(Debug)]
struct FenceQueueData {
    /// `FenceSemState` for each internal queue + 1 (for external).
    wait_states: Vec<FenceWaitState>,
    events: Vec<vk::Event>,
    // TODO
}

#[derive(Debug)]
enum FenceWaitState {
    /// The corresponding internal queue is not allowed to wait on this fence.
    Unavailable,

    /// Should wait for this fence by `vkCmdWaitEvents`.
    Event {
        /// Specifies an index into `FenceQueueData::events`.
        event: usize,
        // TODO: these needs review
        /// Specifies the pipeline stages and access types to include in the
        /// first synchronization/access scope.
        src_scope: (core::PipelineStageFlags, core::AccessTypeFlags),
        /// The pipeline stages and access types that were already included
        /// in the second synchronization/access scope.
        dst_scope: (core::PipelineStageFlags, core::AccessTypeFlags),
    },

    /// Should wait for this fence by using this semaphore.
    Semaphore {
        sem: vk::Semaphore,
        /// Indicates the internal queue index by which the semaphore
        /// was/will be signalled.
        signalled_by: usize,
        /// Indicates whether the semaphore signal operation was
        /// already submitted or not.
        submitted: bool,
        /// Indicates whether the semaphore wait operation was already
        /// submitted or not.
        waited_by: bool,
    },
}

impl<T: DeviceRef> Fence<T> {
    pub fn new(queue: &imp::CommandQueue<T>) -> core::Result<Self> {
        unimplemented!()
        /* let q_data = FenceQueueData{ }; // TODO
        Self{
            data: RefEqArc::new(FenceData{
                device: queue.data.device.clone(),
                q_data: TokenLock::new(&queue.data.token, q_data),
            })
        } */
    }
}
