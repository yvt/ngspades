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
use {arg, buffer, cmd, heap, sampler, shader};

/// Implementation of `Device` for Metal.
#[derive(Debug)]
pub struct Device {
    metal_device: OCPtr<metal::MTLDevice>,
    caps: DeviceCaps,
    arg_layout_info: arg::table::ArgLayoutInfo,
}

zangfx_impl_object! { Device: device::Device, ::Debug }

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Device {
    /// Constructs a new `Device` with a supplied `MTLDevice`.
    ///
    /// `metal_device` must not be null. Otherwise, it will panic.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Result<Self> {
        Ok(Self {
            metal_device: OCPtr::new(metal_device).unwrap(),
            caps: DeviceCaps::new(metal_device),
            arg_layout_info: arg::table::ArgLayoutInfo::new(metal_device)?,
        })
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
        unsafe { Box::new(cmd::queue::CmdQueueBuilder::new(self.metal_device())) }
    }

    fn build_heap(&self) -> Box<base::heap::HeapBuilder> {
        unsafe { Box::new(heap::HeapBuilder::new(self.metal_device())) }
    }

    fn build_barrier(&self) -> Box<base::sync::BarrierBuilder> {
        Box::new(cmd::barrier::BarrierBuilder)
    }

    fn build_image(&self) -> Box<base::resources::ImageBuilder> {
        unimplemented!()
    }

    fn build_buffer(&self) -> Box<base::resources::BufferBuilder> {
        Box::new(buffer::BufferBuilder::new())
    }

    fn build_sampler(&self) -> Box<base::sampler::SamplerBuilder> {
        unsafe { Box::new(sampler::SamplerBuilder::new(self.metal_device())) }
    }

    fn build_library(&self) -> Box<base::shader::LibraryBuilder> {
        Box::new(shader::LibraryBuilder::new())
    }

    fn build_arg_table_sig(&self) -> Box<base::arg::ArgTableSigBuilder> {
        unsafe { Box::new(arg::tablesig::ArgTableSigBuilder::new(self.metal_device())) }
    }

    fn build_root_sig(&self) -> Box<base::arg::RootSigBuilder> {
        Box::new(arg::rootsig::RootSigBuilder::new())
    }

    fn build_arg_pool(&self) -> Box<base::arg::ArgPoolBuilder> {
        unsafe {
            Box::new(arg::table::ArgPoolBuilder::new(
                self.metal_device(),
                self.arg_layout_info,
            ))
        }
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

    fn destroy_buffer(&self, obj: &handles::Buffer) -> Result<()> {
        let our_buffer: &buffer::Buffer = obj.downcast_ref().expect("bad buffer type");
        unsafe {
            our_buffer.destroy();
        }
        Ok(())
    }

    fn destroy_sampler(&self, obj: &handles::Sampler) -> Result<()> {
        let our_sampler: &sampler::Sampler = obj.downcast_ref().expect("bad sampler type");
        unsafe {
            our_sampler.destroy();
        }
        Ok(())
    }

    fn get_memory_req(&self, _obj: handles::ResourceRef) -> Result<base::resources::MemoryReq> {
        unimplemented!()
    }

    fn update_arg_tables(
        &self,
        arg_table_sig: &handles::ArgTableSig,
        updates: &[(&handles::ArgTable, &[device::ArgUpdateSet])],
    ) -> Result<()> {
        let our_sig: &arg::tablesig::ArgTableSig = arg_table_sig
            .downcast_ref()
            .expect("bad argument table signature type");
        our_sig.update_arg_tables(updates)
    }
}
