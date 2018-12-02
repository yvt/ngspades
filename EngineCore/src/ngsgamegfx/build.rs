//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate prebuild_glslang;

fn main() {
    prebuild_glslang::Config::new()
        .file("src/testpass/testpass.frag")
        .flag("-V")
        .compile("testpass.frag.spv");

    prebuild_glslang::Config::new()
        .file("src/testpass/testpass.vert")
        .flag("-V")
        .compile("testpass.vert.spv");
}
