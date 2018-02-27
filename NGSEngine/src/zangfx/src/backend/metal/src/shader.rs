//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Library` for Metal.
use std::sync::Arc;
use rspirv::mr;
use spirv_headers;

use base::{handles, shader};
use common::{Error, ErrorKind, Result};

// TODO: recycle fences after use

/// Implementation of `LibraryBuilder` for Metal.
#[derive(Debug, Default, Clone)]
pub struct LibraryBuilder {
    spirv_code: Option<Vec<u32>>,
}

zangfx_impl_object! { LibraryBuilder: shader::LibraryBuilder, ::Debug }

impl LibraryBuilder {
    pub fn new() -> Self {
        Default::default()
    }
}

impl shader::LibraryBuilder for LibraryBuilder {
    fn spirv_code(&mut self, v: &[u32]) -> &mut shader::LibraryBuilder {
        self.spirv_code = Some(Vec::from(v));
        self
    }

    fn build(&mut self) -> Result<handles::Library> {
        let spirv_code = self.spirv_code
            .clone()
            .ok_or(Error::new(ErrorKind::InvalidUsage))?;
        Library::new(spirv_code).map(handles::Library::new)
    }
}

/// Implementation of `Library` for Metal.
#[derive(Debug, Clone)]
pub struct Library {
    data: Arc<LibraryData>,
}

zangfx_impl_handle! { Library, handles::Library }

#[derive(Debug)]
struct LibraryData {
    spirv_code: Vec<u32>,
}

impl Library {
    fn new(spirv_code: Vec<u32>) -> Result<Self> {
        Ok(Self {
            data: Arc::new(LibraryData { spirv_code }),
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
}
