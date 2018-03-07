//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Device` for Vulkan.
use {base, AshDevice};
use common::Result;

/// Unsafe reference to a Vulkan device object that is internally held by
/// `Device`.
///
/// This type is `'static`, but the referent is only guaranteed to live as long
/// as the originating `Device`. It is the application's responsibility to
/// prevent premature release of `Device` (as required by ZanGFX's base
/// interface.)
#[derive(Debug, Clone, Copy)]
pub(super) struct DeviceRef(*const AshDevice);

impl DeviceRef {
    pub fn vk_device(&self) -> &AshDevice {
        unsafe { &*self.0 }
    }
}

/// Implementation of `Device` for Vulkan.
pub struct Device {
    vk_device: Box<AshDevice>,
}

zangfx_impl_object! { Device: base::Device, ::Debug }

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Device {
    /// Construct a `Device` using a given Vulkan device object.
    ///
    /// `Device` does not destroy the given `AshDevice` automatically when
    /// dropped.
    pub unsafe fn new(vk_device: AshDevice) -> Self {
        Self {
            vk_device: Box::new(vk_device),
        }
    }

    pub fn vk_device(&self) -> &AshDevice {
        &self.vk_device
    }

    /// Construct a `DeviceRef`.
    pub(super) unsafe fn new_device_ref(&self) -> DeviceRef {
        DeviceRef(&*self.vk_device)
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
        unimplemented!()
    }

    fn build_cmd_queue(&self) -> Box<base::CmdQueueBuilder> {
        unimplemented!()
    }

    fn build_semaphore(&self) -> Box<base::SemaphoreBuilder> {
        unimplemented!()
    }

    fn build_dynamic_heap(&self) -> Box<base::DynamicHeapBuilder> {
        unimplemented!()
    }

    fn build_dedicated_heap(&self) -> Box<base::DedicatedHeapBuilder> {
        unimplemented!()
    }

    fn build_barrier(&self) -> Box<base::BarrierBuilder> {
        unimplemented!()
    }

    fn build_image(&self) -> Box<base::ImageBuilder> {
        unimplemented!()
    }

    fn build_buffer(&self) -> Box<base::BufferBuilder> {
        unimplemented!()
    }

    fn build_sampler(&self) -> Box<base::SamplerBuilder> {
        unimplemented!()
    }

    fn build_image_view(&self) -> Box<base::ImageViewBuilder> {
        unimplemented!()
    }

    fn build_library(&self) -> Box<base::LibraryBuilder> {
        unimplemented!()
    }

    fn build_arg_table_sig(&self) -> Box<base::ArgTableSigBuilder> {
        unimplemented!()
    }

    fn build_root_sig(&self) -> Box<base::RootSigBuilder> {
        unimplemented!()
    }

    fn build_arg_pool(&self) -> Box<base::ArgPoolBuilder> {
        unimplemented!()
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
        unimplemented!()
    }

    fn destroy_image(&self, _obj: &base::Image) -> Result<()> {
        unimplemented!()
    }

    fn destroy_buffer(&self, _obj: &base::Buffer) -> Result<()> {
        unimplemented!()
    }

    fn destroy_sampler(&self, _obj: &base::Sampler) -> Result<()> {
        unimplemented!()
    }

    fn destroy_image_view(&self, _obj: &base::ImageView) -> Result<()> {
        unimplemented!()
    }

    fn get_memory_req(&self, _obj: base::ResourceRef) -> Result<base::MemoryReq> {
        unimplemented!()
    }

    fn update_arg_tables(
        &self,
        _arg_table_sig: &base::ArgTableSig,
        _updates: &[(&base::ArgTable, &[base::ArgUpdateSet])],
    ) -> Result<()> {
        unimplemented!()
    }
}
