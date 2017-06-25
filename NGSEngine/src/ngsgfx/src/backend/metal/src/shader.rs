//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;
use spirv_cross::{SpirV2Msl, ExecutionModel, VertexAttribute, VertexInputRate};

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
    pub offset: usize,
    pub stride: usize,
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
                msl_offset: attr.offset as u32,
                msl_stride: attr.stride as u32,
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
