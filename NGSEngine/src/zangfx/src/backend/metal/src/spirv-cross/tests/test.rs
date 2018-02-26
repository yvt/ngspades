//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#[macro_use]
extern crate include_data;
extern crate zangfx_spirv_cross;

use zangfx_spirv_cross::{ExecutionModel, ResourceBinding, SpirV2Msl, VertexAttribute,
                         VertexInputRate};

static TEST_FRAG: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/test.frag.spv"));
static TEST_VERT: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/test.vert.spv"));
static TEST2_VERT: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/test2.vert.spv"));

#[test]
fn transpile_frag() {
    let result = SpirV2Msl::new(TEST_FRAG.as_u32_slice())
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Fragment,
            desc_set: 0,
            binding: 0,
            msl_buffer: None,
            msl_sampler: Some(0),
            msl_texture: Some(0),
            msl_arg_buffer: None,
        })
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Fragment,
            desc_set: 0,
            binding: 1,
            msl_buffer: Some(1),
            msl_sampler: None,
            msl_texture: None,
            msl_arg_buffer: None,
        })
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Fragment,
            desc_set: 1,
            binding: 0,
            msl_buffer: Some(0),
            msl_sampler: None,
            msl_texture: None,
            msl_arg_buffer: None,
        })
        .compile()
        .unwrap();
    println!("// Beginning of Generated Code");
    println!("{}", result.msl_code);
    println!("// End of Generated Code");
    assert!(result.msl_code.contains("unif_buffer [[buffer(1)]]"));
    assert!(result.msl_code.contains("stor_buffer [[buffer(0)]]"));
}

#[test]
fn transpile_frag_iab() {
    let result = SpirV2Msl::new(TEST_FRAG.as_u32_slice())
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Fragment,
            desc_set: 0,
            binding: 0,
            msl_buffer: None,
            msl_sampler: Some(2),
            msl_texture: Some(3),
            msl_arg_buffer: Some(0),
        })
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Fragment,
            desc_set: 0,
            binding: 1,
            msl_buffer: Some(1),
            msl_sampler: None,
            msl_texture: None,
            msl_arg_buffer: Some(0),
        })
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Fragment,
            desc_set: 1,
            binding: 0,
            msl_buffer: Some(0),
            msl_sampler: None,
            msl_texture: None,
            msl_arg_buffer: Some(0),
        })
        .compile()
        .unwrap();
    println!("// Beginning of Generated Code");
    println!("{}", result.msl_code);
    println!("// End of Generated Code");
    assert!(result.msl_code.contains("unif_texture [[id(3)]]"));
    assert!(result.msl_code.contains("unif_textureSmplr [[id(2)]]"));
    assert!(result.msl_code.contains("unif_buffer [[id(1)]]"));
    assert!(result.msl_code.contains("stor_buffer [[id(0)]]"));
    assert!(result.msl_code.contains(".unif_texture"));
    assert!(result.msl_code.contains(".unif_textureSmplr"));
    assert!(result.msl_code.contains(".unif_buffer"));
    assert!(result.msl_code.contains(".stor_buffer"));
}

#[test]
fn transpile_vert() {
    let result = SpirV2Msl::new(TEST_VERT.as_u32_slice())
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Vertex,
            desc_set: 0,
            binding: 0,
            msl_buffer: None,
            msl_sampler: Some(0),
            msl_texture: Some(0),
            msl_arg_buffer: None,
        })
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Vertex,
            desc_set: 0,
            binding: 1,
            msl_buffer: Some(1),
            msl_sampler: None,
            msl_texture: None,
            msl_arg_buffer: None,
        })
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Vertex,
            desc_set: 1,
            binding: 0,
            msl_buffer: Some(0),
            msl_sampler: None,
            msl_texture: None,
            msl_arg_buffer: None,
        })
        .add_vertex_attribute(&VertexAttribute {
            location: 2,
            msl_buffer: 4,
            msl_offset: 12,
            msl_stride: 128,
            input_rate: VertexInputRate::Instance,
        })
        .compile()
        .unwrap();
    println!("// Beginning of Generated Code");
    println!("{}", result.msl_code);
    println!("// End of Generated Code");
    assert!(result.msl_code.contains("hoge [[attribute(2)]]"));
    assert!(result.msl_code.contains("piyo [[user(locn3)]]"));
    assert!(result.msl_code.contains("unif_buffer [[buffer(1)]]"));
    assert!(result.msl_code.contains("stor_buffer [[buffer(0)]]"));
}

#[test]
fn transpile_vert2() {
    let result = SpirV2Msl::new(TEST2_VERT.as_u32_slice())
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Vertex,
            desc_set: 0,
            binding: 2,
            msl_buffer: Some(3),
            msl_sampler: None,
            msl_texture: None,
            msl_arg_buffer: None,
        })
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Vertex,
            desc_set: 0,
            binding: 3,
            msl_buffer: Some(4),
            msl_sampler: None,
            msl_texture: None,
            msl_arg_buffer: None,
        })
        .compile()
        .unwrap();
    println!("// Beginning of Generated Code");
    println!("{}", result.msl_code);
    println!("// End of Generated Code");
    assert!(result.msl_code.contains("u_scene_params [[buffer(3)]]"));
    assert!(result.msl_code.contains("u_obj_params [[buffer(4)]]"));
}

#[test]
fn transpile_vert2_iab() {
    let result = SpirV2Msl::new(TEST2_VERT.as_u32_slice())
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Vertex,
            desc_set: 0,
            binding: 2,
            msl_buffer: Some(2),
            msl_sampler: None,
            msl_texture: None,
            msl_arg_buffer: Some(0),
        })
        .bind_resource(&ResourceBinding {
            stage: ExecutionModel::Vertex,
            desc_set: 0,
            binding: 3,
            msl_buffer: Some(0),
            msl_sampler: None,
            msl_texture: None,
            msl_arg_buffer: Some(0),
        })
        .compile()
        .unwrap();
    println!("// Beginning of Generated Code");
    println!("{}", result.msl_code);
    println!("// End of Generated Code");
    assert!(result.msl_code.contains("u_scene_params [[id(2)]]"));
    assert!(result.msl_code.contains("u_obj_params [[id(0)]]"));
    assert!(result.msl_code.contains("[[buffer(0)]]"));
    assert!(result.msl_code.contains(".u_obj_params"));
}

// TODO: see if entry point name other than `main` works
