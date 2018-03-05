//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate prebuild_glslang;

fn main() {
    prebuild_glslang::Config::new()
        .file("src/backend_tests/compute_null.comp")
        .flag("-V")
        .compile("compute_null.comp.spv");
    prebuild_glslang::Config::new()
        .file("src/backend_tests/compute_conv1.comp")
        .flag("-V")
        .compile("compute_conv1.comp.spv");
    prebuild_glslang::Config::new()
        .file("src/backend_tests/render_null.vert")
        .flag("-V")
        .compile("render_null.vert.spv");
    prebuild_glslang::Config::new()
        .file("src/backend_tests/render_null.frag")
        .flag("-V")
        .compile("render_null.frag.spv");
}
