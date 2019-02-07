//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Library` for Metal.
use std::fmt;
use std::sync::Arc;

use rspirv::mr;
use spirv_headers;
use zangfx_metal_rs as metal;
use zangfx_spirv_cross::{ExecutionModel, SpirV2Msl, VertexAttribute, VertexInputRate};

use zangfx_base::{self as base, shader};
use zangfx_base::{zangfx_impl_handle, zangfx_impl_object};
use zangfx_base::{Error, ErrorKind, Result};

use crate::arg::rootsig::RootSig;
use crate::utils::{nil_error, OCPtr};

// TODO: recycle fences after use

/// Implementation of `LibraryBuilder` for Metal.
#[derive(Debug, Default, Clone)]
pub struct LibraryBuilder {
    spirv_code: Option<Vec<u32>>,
    label: Option<String>,
}

zangfx_impl_object! { LibraryBuilder: dyn shader::LibraryBuilder, dyn crate::Debug, dyn base::SetLabel }

impl LibraryBuilder {
    pub fn new() -> Self {
        Default::default()
    }
}

impl base::SetLabel for LibraryBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
    }
}

impl shader::LibraryBuilder for LibraryBuilder {
    fn spirv_code(&mut self, v: &[u32]) -> &mut dyn shader::LibraryBuilder {
        self.spirv_code = Some(Vec::from(v));
        self
    }

    fn build(&mut self) -> Result<base::LibraryRef> {
        let spirv_code = self.spirv_code.clone().expect("spirv_code");
        Library::new(spirv_code, self.label.clone()).map(base::LibraryRef::new)
    }
}

/// Implementation of `Library` for Metal.
#[derive(Debug, Clone)]
pub struct Library {
    data: Arc<LibraryData>,
}

zangfx_impl_handle! { Library, base::LibraryRef }

#[derive(Debug)]
struct LibraryData {
    spirv_code: Vec<u32>,
    label: Option<String>,
}

impl Library {
    fn new(spirv_code: Vec<u32>, label: Option<String>) -> Result<Self> {
        Ok(Self {
            data: Arc::new(LibraryData { spirv_code, label }),
        })
    }

    pub(crate) fn workgroup_size(&self) -> [u32; 3] {
        let spirv_mod = mr::load_words(&self.data.spirv_code).expect("failed to parse SPIR-V code");

        // Find a variable with the WorkgroupSize built-in decoration
        for anot in spirv_mod.annotations.iter() {
            if anot.operands.len() < 2 {
                continue;
            }
            if let (
                spirv_headers::Op::Decorate,
                &mr::Operand::IdRef(_),
                &mr::Operand::BuiltIn(spirv_headers::BuiltIn::WorkgroupSize),
            ) = (anot.class.opcode, &anot.operands[0], &anot.operands[1])
            {
                unimplemented!();
            }
        }

        // Find OpExecutionMode
        for em in spirv_mod.execution_modes.iter() {
            if em.operands[1] == mr::Operand::ExecutionMode(spirv_headers::ExecutionMode::LocalSize)
            {
                if let (
                    &mr::Operand::LiteralInt32(x),
                    &mr::Operand::LiteralInt32(y),
                    &mr::Operand::LiteralInt32(z),
                ) = (&em.operands[2], &em.operands[3], &em.operands[4])
                {
                    return [x, y, z];
                } else {
                    panic!("invalid OpExecutionMode");
                }
            }
        }

        // Use the default value
        [1, 1, 1]
    }

    pub fn spirv_code(&self) -> &[u32] {
        self.data.spirv_code.as_slice()
    }

    /// Construct a `MTLFunction` based on this `Library`.
    ///
    /// `stage` must specify exactly one shader stage.
    pub(crate) fn new_metal_function<T>(
        &self,
        entry_point: &str,
        stage: shader::ShaderStageFlags,
        root_sig: &RootSig,
        vertex_attrs: T,
        metal_device: metal::MTLDevice,
        pipeline_name: &Option<String>,
    ) -> Result<OCPtr<metal::MTLFunction>>
    where
        T: Iterator<Item = ShaderVertexAttrInfo>,
    {
        assert!(!metal_device.is_null());

        let mut s2m = SpirV2Msl::new(self.spirv_code());

        let model = [
            (shader::ShaderStageFlags::FRAGMENT, ExecutionModel::Fragment),
            (shader::ShaderStageFlags::VERTEX, ExecutionModel::Vertex),
            (shader::ShaderStageFlags::COMPUTE, ExecutionModel::GLCompute),
        ]
        .iter()
        .cloned()
        .find(|(x, _)| x == &stage)
        .unwrap()
        .1;

        root_sig.setup_spirv2msl(&mut s2m, model);

        for attr in vertex_attrs {
            s2m.add_vertex_attribute(&VertexAttribute {
                location: attr.binding as u32,
                msl_buffer: attr.msl_buffer_index as u32,
                msl_offset: attr.offset,
                msl_stride: attr.stride,
                input_rate: match attr.input_rate {
                    metal::MTLVertexStepFunction::PerVertex => VertexInputRate::Vertex,
                    metal::MTLVertexStepFunction::PerInstance => VertexInputRate::Instance,
                    _ => unreachable!(),
                },
            });
        }

        let s2m_output = s2m.compile().map_err(|e| {
            Error::with_detail(ErrorKind::Other, ShaderTranspilationFailed { reason: e })
        })?;
        let code = s2m_output.msl_code;

        let options = unsafe { OCPtr::from_raw(metal::MTLCompileOptions::alloc().init()) }.unwrap();
        options.set_language_version(metal::MTLLanguageVersion::V2_0);

        let lib = OCPtr::new(
            metal_device
                .new_library_with_source(&code, *options)
                .map_err(|e| {
                    Error::with_detail(
                        ErrorKind::Other,
                        ShaderCompilationFailed {
                            reason: e,
                            code: code.clone(),
                        },
                    )
                })?,
        )
        .unwrap();

        if self.data.label.is_some() || pipeline_name.is_some() {
            let pipeline_name = pipeline_name
                .as_ref()
                .map(String::as_str)
                .unwrap_or("(none)");
            let library_name = self
                .data
                .label
                .as_ref()
                .map(String::as_str)
                .unwrap_or("(none)");
            let label = format!("{}: {}", pipeline_name, library_name);
            lib.set_label(&label);
        }

        let fn_name: &str = if entry_point == "main" {
            // `main` is renamed automatically by SPIRV-Cross (probably) because
            // C++11 (which Metal Shading Language is based on) treats a function
            // named `main` in a special way
            "main0"
        } else {
            entry_point
        };

        OCPtr::new(lib.get_function(fn_name))
            .ok_or_else(|| nil_error("MTLLibrary newFunctionWithName:"))
    }
}

#[derive(Debug, Clone)]
struct ShaderTranspilationFailed {
    reason: String,
}

impl ::std::error::Error for ShaderTranspilationFailed {
    fn description(&self) -> &str {
        "failed to transpile a shader code"
    }
}

impl fmt::Display for ShaderTranspilationFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to transpile a shader code due to the following reason: {}",
            &self.reason
        )
    }
}

#[derive(Debug, Clone)]
struct ShaderCompilationFailed {
    reason: String,
    code: String,
}

impl ::std::error::Error for ShaderCompilationFailed {
    fn description(&self) -> &str {
        "failed to compile the transpiled MSL code"
    }
}

impl fmt::Display for ShaderCompilationFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to compile the transpiled MSL code due to the following reason: {}\n\
             \n\
             The transpiled code is shown below:\n{}",
            &self.reason, &self.code
        )
    }
}

/// Vertex attribute information provided to `ShaderModule::get_function`.
pub(crate) struct ShaderVertexAttrInfo {
    crate binding: usize,
    crate msl_buffer_index: usize,
    crate offset: u32,
    crate stride: u32,
    crate input_rate: metal::MTLVertexStepFunction,
}
