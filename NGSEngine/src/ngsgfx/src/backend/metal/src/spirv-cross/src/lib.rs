//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! SPIRV-Cross
//! ===========
//!
//! Contains a subset of SPIRV-Cross (commit: 3ab1700) and Rust binding.

extern crate libc;

mod ll;
mod spirv2msl;

pub use spirv2msl::*;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ExecutionModel {
    Vertex = ll::SpirVCrossExecutionModelVertex,
    TessellationControl = ll::SpirVCrossExecutionModelTessellationControl,
    TessellationEvaluation = ll::SpirVCrossExecutionModelTessellationEvaluation,
    Geometry = ll::SpirVCrossExecutionModelGeometry,
    Fragment = ll::SpirVCrossExecutionModelFragment,
    GLCompute = ll::SpirVCrossExecutionModelGLCompute,
    Kernel = ll::SpirVCrossExecutionModelKernel,
}

impl ExecutionModel {
    fn as_ll(self) -> ll::SpirVCrossExecutionModel {
        self as ll::SpirVCrossExecutionModel
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum VertexInputRate {
    Vertex = ll::SpirVCrossVertexInputRateVertex,
    Instance = ll::SpirVCrossVertexInputRateInstance,
}

impl VertexInputRate {
    fn as_ll(self) -> ll::SpirVCrossVertexInputRate {
        self as ll::SpirVCrossVertexInputRate
    }
}

pub type Result<T> = ::std::result::Result<T, String>;


