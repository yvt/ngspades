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
use tokenlock::{TokenLock, Token};

use {RefEqArc, DeviceRef, AshDevice, translate_generic_error_unwrap};
use imp;
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
    pub semaphores: Vec<Option<vk::Semaphore>>,
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
        ///
        /// `None` if it was signaled by an external entity.
        signalled_by: Option<u32>,
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
        let q_data = FenceQueueData {
            wait_states,
            mutex: ResourceMutex::new(l_data, false),
        };
        let data = FenceData {
            device_ref: queue.device_ref().clone(),
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
        Self::new(
            queue,
            queue
                .device_config()
                .engine_queue_mappings
                .internal_queues_for_engines(desc.wait_engines),
            0,
        )
    }

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

    /// lock the fence for internal state manipulation.
    ///
    /// `queue_lock` must originate from the same `CommandQueue` as the one
    /// this fence was created from. The check operation is considerably fast
    /// compared to usual lock operations.
    pub fn lock<'a: 'b, 'b>(
        &'a self,
        queue_lock: &'b mut imp::CommandQueueLockGuard<T>,
    ) -> FenceLockGuard<'b, T> {
        FenceLockGuard(self, queue_lock.token())
    }
}

impl<T: DeviceRef> FenceQueueData<T> {
    pub(super) fn get_semaphore(&self, dest_iq: usize) -> vk::Semaphore {
        self.mutex.get_host_read().semaphores[dest_iq].unwrap()
    }
}

/// Provides access to the fence state.
#[derive(Debug)]
pub struct FenceLockGuard<'a, T: DeviceRef>(&'a Fence<T>, &'a mut Token);

impl<'a, T: DeviceRef> FenceLockGuard<'a, T> {
    fn fqd(&mut self) -> &mut FenceQueueData<T> {
        self.0.queue_data_write(self.1)
    }

    /// Retrieve the semaphore to be waited by the specified internal queue as well
    /// as a `bool` value indicating whether the semaphore is signaled, or has a pending
    /// signal operation previously submitted for execution.
    ///
    /// `dest_iq` must not be the internal queue that updated the fence for the last time.
    pub fn get_internal_queue_semaphore(
        &mut self,
        dest_iq: usize,
    ) -> Option<(vk::Semaphore, bool)> {
        assert!(dest_iq < self.0.data.num_iqs);

        let fqd: &FenceQueueData<_> = self.fqd();
        if let Some(sem) = fqd.mutex.get_host_read().semaphores[dest_iq] {
            Some((
                sem,
                match fqd.wait_states[dest_iq] {
                    FenceWaitState::Semaphore { .. } => true,
                    FenceWaitState::PipelineBarrier { .. } => {
                        panic!("this internal queue must be waited using a pipeline barrier")
                    }
                    FenceWaitState::Ready => false,
                    _ => unreachable!(),
                },
            ))
        } else {
            None
        }
    }

    /// Retrieve the semaphore to be waited by the specified external entity as well
    /// as a `bool` value indicating whether the semaphore is signaled, or has a pending
    /// signal operation previously submitted for execution.
    pub fn get_external_semaphore(&mut self, ex: usize) -> (vk::Semaphore, bool) {
        let i = ex + self.0.data.num_iqs;

        let fqd: &FenceQueueData<_> = self.fqd();
        (
            fqd.mutex.get_host_read().semaphores[i].unwrap(),
            match fqd.wait_states[i] {
                FenceWaitState::Semaphore { .. } => true,
                FenceWaitState::Ready => false,
                _ => unreachable!(),
            },
        )
    }

    pub fn num_external_destinations(&mut self) -> usize {
        let num_iqs = self.0.data.num_iqs;
        let fqd: &FenceQueueData<_> = self.fqd();
        fqd.wait_states.len() - num_iqs
    }

    /// Cause the next fence wait operation from the specified internal queue to
    /// wait on the corresponding semaphore.
    ///
    /// An example of the intended usage is shown below:
    ///
    /// ```text
    /// let mut queue_guard = queue.lock();
    /// let mut fence_guard = fence.lock(&mut queue_guard);
    /// let mut sems = Vec::new();
    /// for i in 0 .. num_internal_queues {
    ///     if let Some((sem, signaled)) = fence_guard.get_internal_queue_semaphore(i) {
    ///         assert!(!signaled); // or you can insert a dummy
    ///                             // batch to unsignal this semaphore
    ///         fence_guard.signal_internal_queue_semaphore(i);
    ///         sems.push(sem);
    ///     }
    /// }
    /// for i in 0 .. fence_guard.num_external_destinations() {
    ///     let (sem, signaled) = fence_guard.get_external_semaphore(i);
    ///     assert!(!signaled); // or you can insert a dummy
    ///                         // batch to unsignal this semaphore
    ///     fence_guard.signal_external_semaphore(i);
    ///     sems.push(sem);
    /// }
    ///
    /// // [submit some command that signals the set of semaphores `sems`]
    /// ```
    pub unsafe fn signal_internal_queue_semaphore(&mut self, dest_iq: usize) {
        assert!(dest_iq < self.0.data.num_iqs);

        let mut fqd: &mut FenceQueueData<_> = self.fqd();
        fqd.mutex.get_host_read().semaphores[dest_iq].unwrap();

        match fqd.wait_states[dest_iq] {
            FenceWaitState::Semaphore { .. } => panic!("the semaphore is already signaled"),
            FenceWaitState::PipelineBarrier { .. } |
            FenceWaitState::Ready => {
                fqd.wait_states[dest_iq] = FenceWaitState::Semaphore { signalled_by: None };
            }
            _ => unreachable!(),
        }
    }

    /// Cause the next fence wait operation from the specified external entity to
    /// wait on the corresponding semaphore.
    ///
    /// See [`signal_internal_queue_semaphore`] for the usage.
    ///
    /// [`signal_internal_queue_semaphore`]: #tymethod.signal_internal_queue_semaphore
    pub unsafe fn signal_external_semaphore(&mut self, ex: usize) {
        let i = ex + self.0.data.num_iqs;

        let mut fqd: &mut FenceQueueData<_> = self.fqd();
        fqd.mutex.get_host_read().semaphores[i].unwrap();

        match fqd.wait_states[i] {
            FenceWaitState::Semaphore { .. } => panic!("the semaphore is already signaled"),
            FenceWaitState::PipelineBarrier { .. } |
            FenceWaitState::Ready => {
                fqd.wait_states[i] = FenceWaitState::Semaphore { signalled_by: None };
            }
            _ => unreachable!(),
        }
    }

    /// Mark that the specified semaphore was unsignaled by an external entity.
    ///
    /// An example of the intended usage is shown below:
    ///
    /// ```text
    /// let mut queue_guard = queue.lock();
    /// let mut fence_guard = fence.lock(&mut queue_guard);
    /// let (sem, signaled) = fence_guard.get_external_semaphore(i);
    /// assert!(signaled);
    /// // [submit some command that waits on the semaphore `sem`]
    /// fence_guard.unsignal_external_semaphore(i);
    /// ```
    pub unsafe fn unsignal_external_semaphore(&mut self, ex: usize) {
        let i = ex + self.0.data.num_iqs;

        let mut fqd: &mut FenceQueueData<_> = self.fqd();
        fqd.mutex.get_host_read().semaphores[i].unwrap();

        match fqd.wait_states[i] {
            FenceWaitState::Semaphore { .. } => {
                fqd.wait_states[i] = FenceWaitState::Ready;
            }
            FenceWaitState::PipelineBarrier { .. } |
            FenceWaitState::Ready => panic!("invalid state"),
            _ => unreachable!(),
        }
    }
}
