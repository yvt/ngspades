//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate spirv_cross;
#[macro_use]
extern crate include_data;

use spirv_cross::{SpirV2Msl, ResourceBinding, ExecutionModel};

static TEST_FRAG: include_data::DataView = include_data!(concat!(env!("OUT_DIR"), "/test.frag.spv"));

#[test]
fn transpile() {
    let result = SpirV2Msl::new(TEST_FRAG.as_u32_slice())
        .bind_resource(&ResourceBinding {
                            stage: ExecutionModel::Vertex,
                            desc_set: 0,
                            binding: 0,
                            msl_buffer: None,
                            msl_sampler: Some(0),
                            msl_texture: Some(0),
                        })
        .bind_resource(&ResourceBinding {
                            stage: ExecutionModel::Vertex,
                            desc_set: 0,
                            binding: 0,
                            msl_buffer: Some(1),
                            msl_sampler: None,
                            msl_texture: None,
                        })
        .compile()
        .unwrap();
    println!("// Beginning of Generated Code");
    println!("{}", result.msl_code);
    println!("// End of Generated Code");
}

// TODO: see if entry point name other than `main` works
