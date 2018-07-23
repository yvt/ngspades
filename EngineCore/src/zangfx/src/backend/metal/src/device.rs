//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Device` for Metal.
use zangfx_base::{self as base, device, Result};
use zangfx_base::{interfaces, vtable_for, zangfx_impl_object};
use zangfx_metal_rs as metal;

use crate::limits::DeviceCaps;
use crate::utils::OCPtr;
use crate::{
    arg, buffer, cmd, computepipeline, heap, image, renderpass, renderpipeline, sampler, shader,
};

/// Implementation of `Device` for Metal.
#[derive(Debug)]
pub struct Device {
    metal_device: OCPtr<metal::MTLDevice>,
    caps: DeviceCaps,
    arg_layout_info: arg::table::ArgLayoutInfo,
}

zangfx_impl_object! { Device: dyn device::Device, dyn crate::Debug }

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

    /// Constructs a new `Device` with the preferred system default Metal
    /// device.
    pub unsafe fn new_system_default() -> Result<Self> {
        use crate::utils::nil_error;
        let metal_device = metal::create_system_default_device();
        if metal_device.is_null() {
            Err(nil_error("MTLCreateSystemDefaultDevice"))
        } else {
            Self::new(metal_device)
        }
    }

    pub fn metal_device(&self) -> metal::MTLDevice {
        *self.metal_device
    }
}

impl device::Device for Device {
    fn caps(&self) -> &dyn base::limits::DeviceCaps {
        &self.caps
    }

    fn global_heap(&self) -> &base::HeapRef {
        unimplemented!()
    }

    fn build_cmd_queue(&self) -> base::command::CmdQueueBuilderRef {
        unsafe { Box::new(cmd::queue::CmdQueueBuilder::new(self.metal_device())) }
    }

    fn build_dynamic_heap(&self) -> base::heap::DynamicHeapBuilderRef {
        unsafe { Box::new(heap::HeapBuilder::new(self.metal_device())) }
    }

    fn build_dedicated_heap(&self) -> base::heap::DedicatedHeapBuilderRef {
        unsafe { Box::new(heap::HeapBuilder::new(self.metal_device())) }
    }

    fn build_image(&self) -> base::resources::ImageBuilderRef {
        unsafe { Box::new(image::ImageBuilder::new(self.metal_device())) }
    }

    fn build_buffer(&self) -> base::resources::BufferBuilderRef {
        unsafe { Box::new(buffer::BufferBuilder::new(self.metal_device())) }
    }

    fn build_sampler(&self) -> base::sampler::SamplerBuilderRef {
        unsafe { Box::new(sampler::SamplerBuilder::new(self.metal_device())) }
    }

    fn build_library(&self) -> base::shader::LibraryBuilderRef {
        Box::new(shader::LibraryBuilder::new())
    }

    fn build_arg_table_sig(&self) -> base::arg::ArgTableSigBuilderRef {
        unsafe { Box::new(arg::tablesig::ArgTableSigBuilder::new(self.metal_device())) }
    }

    fn build_root_sig(&self) -> base::arg::RootSigBuilderRef {
        Box::new(arg::rootsig::RootSigBuilder::new())
    }

    fn build_arg_pool(&self) -> base::arg::ArgPoolBuilderRef {
        unsafe {
            Box::new(arg::table::ArgPoolBuilder::new(
                self.metal_device(),
                self.arg_layout_info,
            ))
        }
    }

    fn build_render_pass(&self) -> base::pass::RenderPassBuilderRef {
        Box::new(renderpass::RenderPassBuilder::new())
    }

    fn build_render_target_table(&self) -> base::pass::RenderTargetTableBuilderRef {
        unsafe {
            Box::new(renderpass::RenderTargetTableBuilder::new(
                self.metal_device(),
            ))
        }
    }

    fn build_render_pipeline(&self) -> base::pipeline::RenderPipelineBuilderRef {
        unsafe {
            Box::new(renderpipeline::RenderPipelineBuilder::new(
                self.metal_device(),
            ))
        }
    }

    fn build_compute_pipeline(&self) -> base::pipeline::ComputePipelineBuilderRef {
        unsafe {
            Box::new(computepipeline::ComputePipelineBuilder::new(
                self.metal_device(),
            ))
        }
    }

    fn update_arg_tables(
        &self,
        arg_table_sig: &base::ArgTableSigRef,
        updates: &[(
            (&base::ArgPoolRef, &base::ArgTableRef),
            &[device::ArgUpdateSet],
        )],
    ) -> Result<()> {
        let our_sig: &arg::tablesig::ArgTableSig = arg_table_sig
            .downcast_ref()
            .expect("bad argument table signature type");
        our_sig.update_arg_tables(updates)
    }

    fn autorelease_pool_scope_core(&self, cb: &mut dyn FnMut(&mut dyn device::AutoreleasePool)) {
        struct AutoreleasePool(Option<OCPtr<metal::NSAutoreleasePool>>);

        impl device::AutoreleasePool for AutoreleasePool {
            fn drain(&mut self) {
                self.0 = None;
                self.0 = Some(unsafe {
                    OCPtr::from_raw(metal::NSAutoreleasePool::alloc().init()).unwrap()
                });
            }
        }

        let mut op = AutoreleasePool(Some(unsafe {
            OCPtr::from_raw(metal::NSAutoreleasePool::alloc().init()).unwrap()
        }));
        cb(&mut op)
    }
}
