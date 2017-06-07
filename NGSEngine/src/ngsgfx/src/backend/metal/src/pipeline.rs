//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use super::ref_hash;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct GraphicsPipeline {}

impl core::GraphicsPipeline for GraphicsPipeline {}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ComputePipeline {}

impl core::ComputePipeline for ComputePipeline {
    fn max_num_workgroup_invocations(&self) -> u32 {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct StencilState {}

impl core::StencilState for StencilState {

}