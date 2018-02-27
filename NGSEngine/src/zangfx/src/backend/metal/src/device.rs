//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Device` for Metal.
use {base, metal};
use base::{device, handles};
use common::Result;

use utils::OCPtr;
use limits::DeviceCaps;
use {cmd, shader};

/// Implementation of `Device` for Metal.
#[derive(Debug)]
pub struct Device {
    metal_device: OCPtr<metal::MTLDevice>,
    caps: DeviceCaps,
}

zangfx_impl_object! { Device: device::Device, ::Debug }

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Device {
    /// Constructs a new `Device` with a supplied `MTLDevice`.
    ///
    /// `metal_device` must not be null. Otherwise, it will panic.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device: OCPtr::new(metal_device).unwrap(),
            caps: DeviceCaps::new(metal_device),
        }
    }

    pub fn metal_device(&self) -> metal::MTLDevice {
        *self.metal_device
    }
}

impl device::Device for Device {
    fn caps(&self) -> &base::limits::DeviceCaps {
        &self.caps
    }

    fn build_cmd_queue(&self) -> Box<base::command::CmdQueueBuilder> {
        unimplemented!()
    }

    fn build_heap(&self) -> Box<base::heap::HeapBuilder> {
        unimplemented!()
    }

    fn build_barrier(&self) -> Box<base::sync::BarrierBuilder> {
        Box::new(cmd::barrier::BarrierBuilder)
    }

    fn build_image(&self) -> Box<base::resources::ImageBuilder> {
        unimplemented!()
    }

    fn build_buffer(&self) -> Box<base::resources::BufferBuilder> {
        unimplemented!()
    }

    fn build_sampler(&self) -> Box<base::sampler::SamplerBuilder> {
        unimplemented!()
    }

    fn build_library(&self) -> Box<base::shader::LibraryBuilder> {
        Box::new(shader::LibraryBuilder::new())
    }

    fn build_arg_table_sig(&self) -> Box<base::arg::ArgTableSigBuilder> {
        unimplemented!()
    }

    fn build_root_sig(&self) -> Box<base::arg::RootSigBuilder> {
        unimplemented!()
    }

    fn build_arg_pool(&self) -> Box<base::arg::ArgPoolBuilder> {
        unimplemented!()
    }

    fn build_render_pass(&self) -> Box<base::pass::RenderPassBuilder> {
        unimplemented!()
    }

    fn build_rt_table(&self) -> Box<base::pass::RtTableBuilder> {
        unimplemented!()
    }

    fn build_compute_pipeline(&self) -> Box<base::pipeline::ComputePipelineBuilder> {
        unimplemented!()
    }

    fn destroy_image(&self, _obj: &handles::Image) -> Result<()> {
        unimplemented!()
    }

    fn destroy_buffer(&self, _obj: &handles::Buffer) -> Result<()> {
        unimplemented!()
    }

    fn destroy_sampler(&self, _obj: &handles::Sampler) -> Result<()> {
        unimplemented!()
    }

    fn get_memory_req(&self, _obj: handles::ResourceRef) -> Result<base::resources::MemoryReq> {
        unimplemented!()
    }
}
