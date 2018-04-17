//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#[macro_use]
extern crate include_data;
extern crate zangfx_spirv_cross;

use zangfx_spirv_cross::{ExecutionModel, ResourceBinding, SpirV2Msl};

static TEST_FRAG: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/depthtex.frag.spv"));

#[test]
fn transpile_frag_depthtex() {
    let result = SpirV2Msl::new(TEST_FRAG.as_u32_slice())
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Fragment,
            desc_set: 0,
            binding: 0,
            msl_buffer: None,
            msl_sampler: Some(0),
            msl_texture: Some(0),
            msl_arg_buffer: None,
            is_depth_texture: true,
        })
        .compile()
        .unwrap();
    println!("// Beginning of Generated Code");
    println!("{}", result.msl_code);
    println!("// End of Generated Code");
    assert!(result.msl_code.contains("depth2d<float>"));
}
