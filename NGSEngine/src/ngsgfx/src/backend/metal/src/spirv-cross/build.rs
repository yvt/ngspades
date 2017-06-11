//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate gcc;
extern crate prebuild_glslang;

fn main() {
    gcc::Config::new()
        .cpp(true)
        .flag("-std=c++11") // TODO: support MSVC!
        .file("libspirvcross/spirv_cfg.cpp")
        .file("libspirvcross/spirv_cross.cpp")
        .file("libspirvcross/spirv_glsl.cpp")
        .file("libspirvcross/spirv_msl.cpp")
        .file("binding/spirv2msl.cpp")
        .compile("libspirvcross.a");

    prebuild_glslang::Config::new()
        .file("tests/test.frag")
        .flag("-V")
        .compile("test.frag.spv");
}
