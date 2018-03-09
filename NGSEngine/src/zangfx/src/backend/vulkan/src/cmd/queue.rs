//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdQueue` for Vulkan.
use std::sync::Arc;
use ash::vk;
use ash::version::*;
use parking_lot::Mutex;

use base;
use common::{Error, ErrorKind, Result};
use device::DeviceRef;

use limits::DeviceConfig;

use super::monitor::Monitor;

#[derive(Debug)]
pub(crate) struct QueuePool {
    pools: Mutex<Vec<Vec<u32>>>,
}

impl QueuePool {
    pub fn new(config: &DeviceConfig) -> Self {
        let ref queues = config.queues;

        let num_qf = queues.iter().map(|&(qf, _)| qf + 1).max().unwrap_or(0);

        let mut pools = vec![Vec::new(); num_qf as usize];
        for &(qf, i) in queues.iter().rev() {
            pools[qf as usize].push(i);
        }

        Self {
            pools: Mutex::new(pools),
        }
    }

    pub fn allocate_queue(&self, queue_family: base::QueueFamily) -> u32 {
        self.pools.lock()[queue_family as usize]
            .pop()
            .expect("out of queues")
    }
}

/// Implementation of `CmdQueueBuilder` for Vulkan.
#[derive(Debug)]
pub struct CmdQueueBuilder {
    device: DeviceRef,
    queue_pool: Arc<QueuePool>,

    max_num_outstanding_cmd_buffers: usize,
    max_num_outstanding_batches: usize,
    queue_family: Option<base::QueueFamily>,
}

zangfx_impl_object! { CmdQueueBuilder: base::CmdQueueBuilder, ::Debug }

impl CmdQueueBuilder {
    pub(crate) unsafe fn new(device: DeviceRef, queue_pool: Arc<QueuePool>) -> Self {
        Self {
            device,
            queue_pool,
            max_num_outstanding_cmd_buffers: 32,
            max_num_outstanding_batches: 8,
            queue_family: None,
        }
    }

    /// Set the maximum number of outstanding command buffers.
    ///
    /// Defaults to `32`.
    pub fn max_num_outstanding_cmd_buffers(&mut self, v: usize) -> &mut Self {
        self.max_num_outstanding_cmd_buffers = v;
        self
    }

    /// Set the maximum number of outstanding batches.
    ///
    /// Defaults to `8`.
    pub fn max_num_outstanding_batches(&mut self, v: usize) -> &mut Self {
        self.max_num_outstanding_batches = v;
        self
    }
}

impl base::CmdQueueBuilder for CmdQueueBuilder {
    fn queue_family(&mut self, v: base::QueueFamily) -> &mut base::CmdQueueBuilder {
        self.queue_family = Some(v);
        self
    }

    fn build(&mut self) -> Result<Box<base::CmdQueue>> {
        if self.max_num_outstanding_cmd_buffers < 1 {
            return Err(Error::with_detail(
                ErrorKind::InvalidUsage,
                "max_num_outstanding_cmd_buffers",
            ));
        }

        if self.max_num_outstanding_batches < 1 {
            return Err(Error::with_detail(
                ErrorKind::InvalidUsage,
                "max_num_outstanding_batches",
            ));
        }

        let queue_family = self.queue_family
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "queue_family"))?;

        let index = self.queue_pool.allocate_queue(queue_family);

        let vk_device = self.device.vk_device();
        let vk_queue = unsafe { vk_device.get_device_queue(queue_family, index) };

        let num_fences = self.max_num_outstanding_batches;

        CmdQueue::new(self.device, vk_queue, num_fences).map(|x| Box::new(x) as _)
    }
}

/// Implementation of `CmdQueue` for Vulkan.
#[derive(Debug)]
pub struct CmdQueue {
    device: DeviceRef,
    vk_queue: vk::Queue,
    monitor: Monitor,
}

zangfx_impl_object! { CmdQueue: base::CmdQueue, ::Debug }

impl CmdQueue {
    fn new(device: DeviceRef, vk_queue: vk::Queue, num_fences: usize) -> Result<Self> {
        Ok(Self {
            device,
            vk_queue,
            monitor: Monitor::new(device, vk_queue, num_fences)?,
        })
    }
}

impl base::CmdQueue for CmdQueue {
    fn new_cmd_buffer(&self) -> Result<Box<base::CmdBuffer>> {
        unimplemented!()
    }

    fn new_fence(&self) -> Result<base::Fence> {
        unimplemented!()
    }

    fn flush(&self) {
        unimplemented!()
    }
}
