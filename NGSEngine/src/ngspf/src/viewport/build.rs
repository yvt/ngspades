//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate prebuild_glslang;

fn main() {
    prebuild_glslang::Config::new()
        .file("src/glsl/composite.frag")
        .flag("-V")
        .compile("composite.frag.spv");

    prebuild_glslang::Config::new()
        .file("src/glsl/composite.vert")
        .flag("-V")
        .compile("composite.vert.spv");
}
