//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;
use spirv_cross::{SpirV2Msl, ExecutionModel, VertexAttribute, VertexInputRate};
use cgmath::Vector3;
use rspirv::mr;
use spirv_headers;

use std::sync::Mutex;

use {RefEqArc, imp, OCPtr};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ShaderModule {
    data: RefEqArc<ShaderModuleData>,
}

#[derive(Debug)]
struct ShaderModuleData {
    spirv_code: Vec<u32>,
    label: Mutex<Option<String>>,
}

impl core::Marker for ShaderModule {
    fn set_label(&self, label: Option<&str>) {
        *self.data.label.lock().unwrap() = label.map(String::from);
    }
}

impl core::ShaderModule for ShaderModule {}

/// Vertex attribute information provided to `ShaderModule::get_function`.
pub(crate) struct ShaderVertexAttributeInfo {
    pub binding: usize,
    pub msl_buffer_index: usize,
    pub offset: u32,
    pub stride: u32,
    pub input_rate: core::VertexInputRate,
}

impl ShaderModule {
    pub(crate) fn new(description: &core::ShaderModuleDescription) -> Self {
        let data = ShaderModuleData {
            spirv_code: Vec::from(description.spirv_code),
            label: Mutex::new(None),
        };
        Self { data: RefEqArc::new(data) }
    }

    pub(crate) fn workgroup_size(&self) -> Vector3<u32> {
        let spirv_mod = mr::load_words(&self.data.spirv_code).expect("failed to parse SPIR-V code");

        // Find a variable with the WorkgroupSize built-in decoration
        for anot in spirv_mod.annotations.iter() {
            if anot.operands.len() < 2 {
                continue;
            }
            if let (spirv_headers::Op::Decorate,
                    &mr::Operand::IdRef(_),
                    &mr::Operand::BuiltIn(spirv_headers::BuiltIn::WorkgroupSize)) =
                (anot.class.opcode, &anot.operands[0], &anot.operands[1])
            {
                unimplemented!();
            }
        }

        // Find OpExecutionMode
        for em in spirv_mod.execution_modes.iter() {
            if em.operands[1] ==
                mr::Operand::ExecutionMode(spirv_headers::ExecutionMode::LocalSize)
            {
                if let (&mr::Operand::LiteralInt32(x),
                        &mr::Operand::LiteralInt32(y),
                        &mr::Operand::LiteralInt32(z)) =
                    (&em.operands[2], &em.operands[3], &em.operands[4])
                {
                    return Vector3::new(x, y, z);
                } else {
                    panic!("invalid OpExecutionMode");
                }
            }
        }

        // Use the default value
        Vector3::new(1, 1, 1)
    }

    pub(crate) fn get_function<T>(
        &self,
        entry_point: &str,
        stage: core::ShaderStage,
        layout: &imp::PipelineLayout,
        device: metal::MTLDevice,
        vertex_attrs: T,
    ) -> OCPtr<metal::MTLFunction>
    where
        T: Iterator<Item = ShaderVertexAttributeInfo>,
    {
        assert!(!device.is_null());

        // Setup the SPIR-V-to-Metal transpiler
        let mut s2m = SpirV2Msl::new(self.data.spirv_code.as_slice());

        let model = match stage {
            core::ShaderStage::Fragment => ExecutionModel::Fragment,
            core::ShaderStage::Vertex => ExecutionModel::Vertex,
            core::ShaderStage::Compute => ExecutionModel::GLCompute,
        };

        layout.setup_spirv2msl(&mut s2m, model);

        for attr in vertex_attrs {
            s2m.add_vertex_attribute(&VertexAttribute {
                location: attr.binding as u32,
                msl_buffer: attr.msl_buffer_index as u32,
                msl_offset: attr.offset,
                msl_stride: attr.stride,
                input_rate: match attr.input_rate {
                    core::VertexInputRate::Vertex => VertexInputRate::Vertex,
                    core::VertexInputRate::Instance => VertexInputRate::Instance,
                },
            });
        }

        let output = s2m.compile().expect(
            "compilation from SPIR-V to MSL has failed",
        );
        let code = output.msl_code;

        let options = unsafe { OCPtr::from_raw(metal::MTLCompileOptions::alloc().init()) }.unwrap();
        options.set_language_version(metal::MTLLanguageVersion::V1_1);

        let lib = OCPtr::new(device.new_library_with_source(&code, *options).expect(
            "compilation of MSL has failed",
        )).unwrap();

        let fn_name: &str = if entry_point == "main" {
            // `main` is renamed automatically by SPIRV-Cross (probably) because
            // C++11 (which Metal Shading Language is based on) treats a function
            // named `main` in a special way
            "main0"
        } else {
            entry_point
        };

        OCPtr::new(lib.get_function(fn_name)).unwrap()
    }
}
