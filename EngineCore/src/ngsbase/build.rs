extern crate walkdir;

use std::env;
use std::path::Path;
use std::process::Command;

use walkdir::WalkDir;

fn main() {
    let ngsbase_path_str = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ngsbase_path = Path::new(&ngsbase_path_str);
    let ngsengine_path = ngsbase_path.parent().unwrap().parent().unwrap();
    let project_path = ngsengine_path.parent().unwrap();

    // Interop code generation tools
    let interopgen_path = project_path.join("Ngs.RustInteropGen");
    if !interopgen_path.exists() {
        panic!("Ngs.RustInteropGen was not found.");
    }
    let interopgen_csproj_path = interopgen_path.join("Ngs.RustInteropGen.csproj");

    // Interface definitions
    let interop_path = project_path.join("Ngs.Engine.Core");

    // Output file
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("interop.rs");

    let st = Command::new("dotnet")
        .args(&[
            "run",
            "-p",
            interopgen_csproj_path.to_str().unwrap(),
            "--",
            "-o",
            dest_path.to_str().unwrap(),
        ]).status()
        .unwrap();

    if !st.success() {
        panic!(
            "Command dotnet run -p \"{}\" -- -o \"{}\" failed with exit code {}",
            interopgen_csproj_path.to_str().unwrap(),
            dest_path.to_str().unwrap(),
            st
        );
    }

    // emit cargo:rerun-if-changed
    for entry in WalkDir::new("src") {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }
    for entry in WalkDir::new(interop_path) {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }
    for entry in WalkDir::new(interopgen_path) {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }
}
