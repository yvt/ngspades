//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ll;
use std::ffi::{CStr, CString};
use {ExecutionModel, Result, VertexInputRate};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct VertexAttribute {
    pub location: u32,
    pub msl_buffer: u32,
    pub msl_offset: u32,
    pub msl_stride: u32,
    pub input_rate: VertexInputRate,
}

impl VertexAttribute {
    fn as_ll(self) -> ll::SpirV2MslVertexAttr {
        ll::SpirV2MslVertexAttr {
            location: self.location,
            msl_buffer: self.msl_buffer,
            msl_offset: self.msl_offset,
            msl_stride: self.msl_stride,
            input_rate: self.input_rate.as_ll(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ResourceBinding {
    pub desc_set: u32,
    pub binding: u32,

    pub msl_buffer: Option<u32>,
    pub msl_texture: Option<u32>,
    pub msl_sampler: Option<u32>,

    /// The index of argument buffer. When specified, `msl_buffer`,
    /// `msl_texture`, and `msl_sampler` point indices into the argument buffer.
    pub msl_arg_buffer: Option<u32>,

    pub stage: ExecutionModel,

    pub is_depth_texture: bool,
}

impl ResourceBinding {
    fn as_ll(self) -> ll::SpirV2MslResourceBinding {
        ll::SpirV2MslResourceBinding {
            desc_set: self.desc_set,
            binding: self.binding,
            msl_buffer: self.msl_buffer.unwrap_or(0),
            msl_texture: self.msl_texture.unwrap_or(0),
            msl_sampler: self.msl_sampler.unwrap_or(0),
            msl_arg_buffer: self.msl_arg_buffer.unwrap_or(<u32>::max_value()),
            stage: self.stage.as_ll(),
            is_depth_texture: if self.is_depth_texture { 1 } else { 0 },
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct IndirectArgument<'a> {
    /// The index in the buffer argument table.
    pub msl_arg_buffer: u32,
    /// The index in the argument buffer.
    pub msl_arg: u32,
    // Metal type qualifier.
    pub msl_type: &'a str,
}

impl<'a> IndirectArgument<'a> {
    fn as_ll(self) -> (ll::SpirV2MslIndirectArgument, CString) {
        let cstr = CString::new(self.msl_type).unwrap();
        (
            ll::SpirV2MslIndirectArgument {
                msl_arg_buffer: self.msl_arg_buffer,
                msl_arg: self.msl_arg,
                msl_type: cstr.as_ptr(),
            },
            cstr,
        )
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SpirV2MslOutput {
    pub msl_code: String,
}

/// SPIR-V to Metal Shading Language transpiler.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SpirV2Msl {
    /// Non-null pointer
    obj: *mut ll::SpirV2Msl,
}

unsafe impl Send for SpirV2Msl {}
unsafe impl Sync for SpirV2Msl {}

impl SpirV2Msl {
    pub fn new(spirv: &[u32]) -> Self {
        unsafe {
            let obj = ll::SpirV2MslCreate(spirv.as_ptr(), spirv.len() as u32);
            assert!(!obj.is_null(), "out of memory");
            Self { obj }
        }
    }

    pub fn add_vertex_attribute(&mut self, va: &VertexAttribute) -> &mut Self {
        let ll = va.as_ll();
        unsafe {
            ll::SpirV2MslAddVertexAttr(self.obj, &ll as *const ll::SpirV2MslVertexAttr);
        }
        self
    }

    pub fn bind_resource(&mut self, rb: &ResourceBinding) -> &mut Self {
        let ll = rb.as_ll();
        unsafe {
            ll::SpirV2MslAddResourceBinding(self.obj, &ll as *const ll::SpirV2MslResourceBinding);
        }
        self
    }

    pub fn add_indirect_argument(&mut self, ia: &IndirectArgument) -> &mut Self {
        let (ll, _cstr) = ia.as_ll();
        unsafe {
            ll::SpirV2MslAddIndirectArgument(self.obj, &ll as *const ll::SpirV2MslIndirectArgument);
        }
        self
    }

    pub fn compile(&mut self) -> Result<SpirV2MslOutput> {
        let success = unsafe { ll::SpirV2MslCompile(self.obj) };
        if success != ll::SpirVCrossBoolFalse {
            let msl_code_c = unsafe { ll::SpirV2MslGetOutputSourceCode(self.obj) };
            assert!(!msl_code_c.is_null());
            let msl_code = unsafe { CStr::from_ptr(msl_code_c) }.to_str().unwrap();

            Ok(SpirV2MslOutput {
                msl_code: String::from(msl_code),
            })
        } else {
            let error_c = unsafe { ll::SpirV2MslGetError(self.obj) };
            assert!(!error_c.is_null());
            let error = unsafe { CStr::from_ptr(error_c) }.to_str().unwrap();
            Err(String::from(error))
        }
    }
}

impl Drop for SpirV2Msl {
    fn drop(&mut self) {
        unsafe {
            ll::SpirV2MslDestroy(self.obj);
        }
    }
}
