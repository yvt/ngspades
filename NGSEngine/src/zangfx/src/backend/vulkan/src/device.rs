//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Device` for Vulkan.
use std::sync::Arc;
use {base, AshDevice};
use {arg, buffer, cmd, heap, limits, pipeline, shader, utils};
use common::Result;

/// Unsafe reference to a Vulkan device object that is internally held by
/// `Device`.
///
/// This type is `'static`, but the referent is only guaranteed to live as long
/// as the originating `Device`. It is the application's responsibility to
/// prevent premature release of `Device` (as required by ZanGFX's base
/// interface.)
#[derive(Debug, Clone, Copy)]
pub(super) struct DeviceRef(*const AshDevice, *const limits::DeviceCaps);

unsafe impl Sync for DeviceRef {}
unsafe impl Send for DeviceRef {}

impl DeviceRef {
    pub fn vk_device(&self) -> &AshDevice {
        unsafe { &*self.0 }
    }

    pub fn caps(&self) -> &limits::DeviceCaps {
        unsafe { &*self.1 }
    }
}

/// Implementation of `Device` for Vulkan.
pub struct Device {
    // These fields are boxed so they can be referenced by `DeviceRef`
    vk_device: Box<AshDevice>,
    caps: Box<limits::DeviceCaps>,
    queue_pool: Arc<cmd::queue::QueuePool>,
}

zangfx_impl_object! { Device: base::Device, ::Debug }

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Device {
    /// Construct a `Device` using a given Vulkan device object and
    /// backend configurations;
    ///
    /// `Device` does not destroy the given `AshDevice` automatically when
    /// dropped.
    ///
    /// Fails and returns `Err(_)` if the configuration fails validation.
    pub unsafe fn new(
        vk_device: AshDevice,
        info: limits::DeviceInfo,
        config: limits::DeviceConfig,
    ) -> Result<Self> {
        let caps = limits::DeviceCaps::new(info, config)?;
        let queue_pool = cmd::queue::QueuePool::new(&caps.config);

        Ok(Self {
            vk_device: Box::new(vk_device),
            caps: Box::new(caps),
            queue_pool: Arc::new(queue_pool),
        })
    }

    pub fn vk_device(&self) -> &AshDevice {
        &self.vk_device
    }

    pub(super) fn caps(&self) -> &limits::DeviceCaps {
        &self.caps
    }

    /// Construct a `DeviceRef` pointing this `Device`.
    pub(super) unsafe fn new_device_ref(&self) -> DeviceRef {
        DeviceRef(&*self.vk_device, &*self.caps)
    }
}

use std::fmt;
impl ::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Device").finish()
    }
}

impl base::Device for Device {
    fn caps(&self) -> &base::DeviceCaps {
        &*self.caps
    }

    fn build_cmd_queue(&self) -> Box<base::CmdQueueBuilder> {
        unsafe {
            Box::new(cmd::queue::CmdQueueBuilder::new(
                self.new_device_ref(),
                self.queue_pool.clone(),
            ))
        }
    }

    fn build_semaphore(&self) -> Box<base::SemaphoreBuilder> {
        unimplemented!()
    }

    fn build_dynamic_heap(&self) -> Box<base::DynamicHeapBuilder> {
        unsafe { Box::new(heap::DynamicHeapBuilder::new(self.new_device_ref())) }
    }

    fn build_dedicated_heap(&self) -> Box<base::DedicatedHeapBuilder> {
        unsafe { Box::new(heap::DedicatedHeapBuilder::new(self.new_device_ref())) }
    }

    fn build_barrier(&self) -> Box<base::BarrierBuilder> {
        unimplemented!()
    }

    fn build_image(&self) -> Box<base::ImageBuilder> {
        unimplemented!()
    }

    fn build_buffer(&self) -> Box<base::BufferBuilder> {
        unsafe { Box::new(buffer::BufferBuilder::new(self.new_device_ref())) }
    }

    fn build_sampler(&self) -> Box<base::SamplerBuilder> {
        unimplemented!()
    }

    fn build_image_view(&self) -> Box<base::ImageViewBuilder> {
        unimplemented!()
    }

    fn build_library(&self) -> Box<base::LibraryBuilder> {
        unsafe { Box::new(shader::LibraryBuilder::new(self.new_device_ref())) }
    }

    fn build_arg_table_sig(&self) -> Box<base::ArgTableSigBuilder> {
        unsafe { Box::new(arg::layout::ArgTableSigBuilder::new(self.new_device_ref())) }
    }

    fn build_root_sig(&self) -> Box<base::RootSigBuilder> {
        unsafe { Box::new(arg::layout::RootSigBuilder::new(self.new_device_ref())) }
    }

    fn build_arg_pool(&self) -> Box<base::ArgPoolBuilder> {
        unsafe { Box::new(arg::pool::ArgPoolBuilder::new(self.new_device_ref())) }
    }

    fn build_render_pass(&self) -> Box<base::RenderPassBuilder> {
        unimplemented!()
    }

    fn build_render_target_table(&self) -> Box<base::RenderTargetTableBuilder> {
        unimplemented!()
    }

    fn build_render_pipeline(&self) -> Box<base::RenderPipelineBuilder> {
        unimplemented!()
    }

    fn build_compute_pipeline(&self) -> Box<base::ComputePipelineBuilder> {
        unsafe { Box::new(pipeline::ComputePipelineBuilder::new(self.new_device_ref())) }
    }

    fn destroy_image(&self, _obj: &base::Image) -> Result<()> {
        unimplemented!()
    }

    fn destroy_buffer(&self, obj: &base::Buffer) -> Result<()> {
        let our_buffer: &buffer::Buffer = obj.downcast_ref().expect("bad buffer type");
        unsafe {
            our_buffer.destroy(self.vk_device());
        }
        Ok(())
    }

    fn destroy_sampler(&self, _obj: &base::Sampler) -> Result<()> {
        unimplemented!()
    }

    fn destroy_image_view(&self, _obj: &base::ImageView) -> Result<()> {
        unimplemented!()
    }

    fn get_memory_req(&self, obj: base::ResourceRef) -> Result<base::MemoryReq> {
        utils::get_memory_req(self.vk_device(), obj)
    }

    fn update_arg_tables(
        &self,
        _arg_table_sig: &base::ArgTableSig,
        _updates: &[(&base::ArgTable, &[base::ArgUpdateSet])],
    ) -> Result<()> {
        unimplemented!()
    }
}
