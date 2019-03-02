//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use flatc_rust;
use std::{path::Path, env};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    println!("cargo:rerun-if-changed=flatbuffers/chunk.fbs");
    flatc_rust::run(flatc_rust::Args {
        inputs: &[Path::new("flatbuffers/chunk.fbs")],
        out_dir,
        ..Default::default()
    })
    .expect("flatc");
}
