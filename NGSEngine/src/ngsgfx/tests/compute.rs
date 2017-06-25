//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

extern crate ngsgfx as gfx;
extern crate cgmath;
#[macro_use]
extern crate include_data;

use gfx::core;
use gfx::prelude::*;

use cgmath::Vector3;

use std::time;

static SPIRV_NULL: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/compute_null.comp.spv"));

trait BackendDispatch {
    fn use_device<B: core::Backend>(self, device: B::Device);
}

#[cfg(target_os = "macos")]
fn try_device_metal<T: BackendDispatch>(d: T) -> Option<T> {
    use gfx::backends::metal::ll::NSObjectProtocol;
    let arp = gfx::backends::metal::ll::NSAutoreleasePool::alloc().init();
    let metal_device = gfx::backends::metal::ll::create_system_default_device();
    let device = gfx::backends::metal::imp::Device::new(metal_device);
    d.use_device::<gfx::backends::metal::Backend>(device);
    unsafe { arp.release(); }
    None
}

#[cfg(not(target_os = "macos"))]
fn try_device_metal<T: BackendDispatch>(d: T) -> Option<T> {
    Some(d)
}

fn find_default_device<T: BackendDispatch>(d: T) {
    let t = Some(d).and_then(try_device_metal);
    if t.is_some() {
        panic!("no backend available -- cannot proceed");
    }
}

#[test]
fn simple() {
    find_default_device(SimpleTest);
}

struct SimpleTest;
impl BackendDispatch for SimpleTest {
    fn use_device<B: core::Backend>(self, device: B::Device) {
        let factory = device.factory();

        let shader_desc = core::ShaderModuleDescription { spirv_code: SPIRV_NULL.as_u32_slice() };
        let shader = factory.make_shader_module(&shader_desc).unwrap();

        let layout_desc = core::PipelineLayoutDescription { descriptor_set_layouts: &[] };
        let layout = factory.make_pipeline_layout(&layout_desc).unwrap();

        let pipeline_desc = core::ComputePipelineDescription {
            label: Some("test compute pipeline: null"),
            shader_stage: core::ShaderStageDescription {
                stage: core::ShaderStage::Compute,
                module: &shader,
                entry_point_name: "main",
            },
            pipeline_layout: &layout,
        };

        let pipeline = factory.make_compute_pipeline(&pipeline_desc).unwrap();

        let queue = device.main_queue();
        let mut cb = queue.make_command_buffer().unwrap();

        cb.begin_encoding();
        cb.begin_compute_pass();
        cb.bind_compute_pipeline(&pipeline);
        cb.dispatch(Vector3::new(1, 1, 1));
        cb.end_pass();
        cb.end_encoding();

        queue
            .submit_commands(
                &[
                    &core::SubmissionInfo {
                        buffers: &[&cb],
                        wait_semaphores: &[],
                        signal_semaphores: &[],
                    },
                ],
                None,
            )
            .unwrap();
        assert_eq!(
            cb.wait_completion(time::Duration::from_secs(1)).unwrap(),
            true
        );
    }
}
