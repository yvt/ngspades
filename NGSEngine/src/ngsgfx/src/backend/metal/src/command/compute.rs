//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use cgmath::Vector3;

use imp::{Backend, CommandBuffer, ComputePipeline};

impl core::ComputeCommandEncoder<Backend> for CommandBuffer {
    fn bind_compute_pipeline(&mut self, pipeline: &ComputePipeline) {
        unimplemented!()
    }
    fn dispatch(&mut self, workgroup_count: Vector3<u32>) {
        unimplemented!()
    }
}