//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use libc::*;


// spirvcross.h
pub type SpirVCrossExecutionModel = u8;

pub const SpirVCrossExecutionModelVertex: SpirVCrossExecutionModel = 0;
pub const SpirVCrossExecutionModelTessellationControl: SpirVCrossExecutionModel = 1;
pub const SpirVCrossExecutionModelTessellationEvaluation: SpirVCrossExecutionModel = 2;
pub const SpirVCrossExecutionModelGeometry: SpirVCrossExecutionModel = 3;
pub const SpirVCrossExecutionModelFragment: SpirVCrossExecutionModel = 4;
pub const SpirVCrossExecutionModelGLCompute: SpirVCrossExecutionModel = 5;
pub const SpirVCrossExecutionModelKernel: SpirVCrossExecutionModel = 6;

pub type SpirVCrossVertexInputRate = u8;

pub const SpirVCrossVertexInputRateVertex: SpirVCrossVertexInputRate = 0;
pub const SpirVCrossVertexInputRateInstance: SpirVCrossVertexInputRate = 1;

pub type SpirVCrossBool = u8;

pub const SpirVCrossBoolTrue: SpirVCrossBool = 1;
pub const SpirVCrossBoolFalse: SpirVCrossBool = 0;

// spirv2msl.h
#[repr(C)]
pub struct SpirV2MslVertexAttr {
    pub location: u32,
    pub msl_buffer: u32,
    pub msl_offset: u32,
    pub msl_stride: u32,
    pub input_rate: SpirVCrossVertexInputRate,
}

#[repr(C)]
pub struct SpirV2MslResourceBinding {
    pub desc_set: u32,
    pub binding: u32,

    pub msl_buffer: u32,
    pub msl_texture: u32,
    pub msl_sampler: u32,

    pub stage: SpirVCrossExecutionModel,
}

pub type SpirV2Msl = c_void;

// spirv2msl.h
extern {
    pub fn SpirV2MslCreate(spirv: *const u32, spirv_count: u32) -> *mut SpirV2Msl;
    pub fn SpirV2MslDestroy(this: *mut SpirV2Msl);
    pub fn SpirV2MslAddVertexAttr(this: *mut SpirV2Msl, vertex_attr: *const SpirV2MslVertexAttr);
    pub fn SpirV2MslAddResourceBinding(this: *mut SpirV2Msl, binding: *const SpirV2MslResourceBinding);
    pub fn SpirV2MslCompile(this: *mut SpirV2Msl) -> SpirVCrossBool;
    pub fn SpirV2MslGetError(this: *mut SpirV2Msl) -> *mut c_char;
    pub fn SpirV2MslGetOutputSourceCode(this: *mut SpirV2Msl) -> *mut c_char;
}
