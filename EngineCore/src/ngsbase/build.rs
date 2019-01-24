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

    // Interface definitions
    let interop_path = project_path.join("Ngs.Engine.Core");

    // Output file
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("interop.rs");

    // Calling `dotnet` directly causes "Unable to load shared library
    // 'libproc'" error similar to <https://github.com/dotnet/corefx/issues/25157>,
    // so we try calling it via the shell.
    // (The actual cause of this problem is unknown.)
    let st;
    match Command::new("/bin/sh")
        .current_dir(&interopgen_path)
        .args(&[
            "-c",
            &format!("dotnet run -- -o \"{}\"", dest_path.to_str().unwrap()),
        ])
        .status()
    {
        Ok(status) => {
            st = status;
        }
        Err(_) => {
            // A shell is unavailable. Probably we are running on Windows.
            st = Command::new("dotnet")
                .current_dir(&interopgen_path)
                .args(&["run", "--", "-o", dest_path.to_str().unwrap()])
                .status()
                .unwrap();
        }
    }

    if !st.success() {
        panic!(
            "Command dotnet run -- -o \"{}\" failed with exit code {}",
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
