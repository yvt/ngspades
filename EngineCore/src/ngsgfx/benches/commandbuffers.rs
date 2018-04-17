//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Measures the throughput of command buffer submission.
#![feature(test)]
extern crate ngsgfx as gfx;
extern crate cgmath;
#[macro_use]
extern crate include_data;
extern crate test;

use test::Bencher;

use gfx::core;
use gfx::prelude::*;

use cgmath::Vector3;

static SPIRV_NULL: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/compute_null.comp.spv"));

fn use_device<B: core::Backend>(b: &mut Bencher, device: B::Device, num_cbs: usize) {
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

    B::autorelease_pool_scope(move |_| {
        let mut cb_ring: Vec<Vec<_>> = (0..5)
            .map(|_| {
                (0..num_cbs)
                    .map(|_| queue.make_command_buffer().unwrap())
                    .collect()
            })
            .collect();

        b.iter(|| {
            B::autorelease_pool_scope(|arp| for i in 0..100 {
                let cb_idx = i % cb_ring.len();
                let ref mut cbs = cb_ring[cb_idx];
                for cb in cbs.iter() {
                    cb.wait_completion().unwrap();
                }
                for cb in cbs.iter_mut() {
                    cb.begin_encoding();
                    cb.begin_compute_pass(core::DeviceEngine::Compute);
                    cb.bind_compute_pipeline(&pipeline);
                    cb.dispatch(Vector3::new(1, 1, 1));
                    cb.end_pass();
                    cb.end_encoding().unwrap();
                }

                let mut cb_refs: Vec<_> = cbs.iter_mut().collect();
                queue.submit_commands(&mut cb_refs[..], None).unwrap();

                arp.drain();
            });
        });
    });
}

fn use_env<K: core::Environment>(b: &mut Bencher, num_cbs: usize) {
    use core::{InstanceBuilder, Instance, DeviceBuilder, Backend};
    <K::Backend as Backend>::autorelease_pool_scope(move |_| {
        let inst_builder = K::InstanceBuilder::new().unwrap();
        let instance = inst_builder.build().unwrap();
        let default_adapter = instance.default_adapter().unwrap();
        let device_builder = instance.new_device_builder(&default_adapter);
        let device = device_builder.build().unwrap();
        use_device::<K::Backend>(b, device, num_cbs);
    });
}

#[bench]
fn cb_throughput_10(b: &mut Bencher) {
    use_env::<gfx::backends::DefaultEnvironment>(b, 10);
}

#[bench]
fn cb_throughput_20(b: &mut Bencher) {
    use_env::<gfx::backends::DefaultEnvironment>(b, 20);
}

#[bench]
fn cb_throughput_40(b: &mut Bencher) {
    use_env::<gfx::backends::DefaultEnvironment>(b, 40);
}
