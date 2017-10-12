//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! A library for build scripts to invoke [glslangvalidator](https://github.com/KhronosGroup/glslang)
//! to compile GLSL source files into SPIR-V binary files.
//!
//! Mostly based on the `gcc` crate (which similarly calls a C/C++ compiler).
//!
//! # Examples
//!
//! ```ignore
//! extern crate prebuild_glslang;
//!
//! fn main() {
//!     prebuild_glslang::Config::new()
//!         .file("twilight-horn.frag")
//!         .compile("twilight-horn.frag.spv")
//! }
//! ```
//!
//! The generated SPIR-V code can be included like this:
//!
//! ```ignore
//! #[macro_use]
//! extern crate include_data;
//!
//! static MY_SPIRV_CODE: include_data::DataView = include_data!(concat!(env!("OUT_DIR"), "/twilight-horn-frag.frag.spv"))
//!
//! fn main() {
//!     let code: &'static [u32] = MY_SPIRV_CODE.as_u32_slice();
//! }
//! ```
//!
//! # Unimplemented Features
//!
//! - Handling of endianness --- [`build_helper`] would be useful
//!
//! [`build_helper`]: https://docs.rs/build-helper/0.1.0/build_helper/enum.Endianness.html
//!
use std::path::{PathBuf, Path};
use std::process::{Command, Stdio, Child};
use std::env;
use std::thread::{self, JoinHandle};
use std::io::{self, BufRead, BufReader, Write};

pub struct Config {
    flags: Vec<String>,
    files: Vec<PathBuf>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            flags: Vec::new(),
            files: Vec::new(),
        }
    }

    pub fn flag(&mut self, flag: &str) -> &mut Self {
        self.flags.push(flag.to_string());
        self
    }

    /// Adds an input file.
    pub fn file<P: AsRef<Path>>(&mut self, p: P) -> &mut Self {
        self.files.push(p.as_ref().to_path_buf());
        self
    }

    pub fn get_glslang_path(&self) -> PathBuf {
        PathBuf::from("glslangValidator").to_path_buf()
    }

    fn get_out_dir(&self) -> PathBuf {
        env::var_os("OUT_DIR").map(PathBuf::from).unwrap()
    }

    /// Run the compiler. A SPIR-V formatted file named `output` will be generated.
    pub fn compile(&self, output: &str) {
        let output_dir = self.get_out_dir();
        let mut cmd = Command::new(self.get_glslang_path());
        cmd.args(self.flags.iter());
        cmd.args(self.files.iter());

        cmd.arg("-o");
        cmd.arg(output_dir.join(output));

        run(&mut cmd, "glslangValidator");
    }
}

fn run(cmd: &mut Command, program: &str) {
    let (mut child, print) = spawn(cmd, program);
    let status = child.wait().expect("failed to wait on child process");
    print.join().unwrap();
    println!("{}", status);
    if !status.success() {
        fail(&format!("command did not execute successfully, got: {}", status));
    }
}

fn spawn(cmd: &mut Command, program: &str) -> (Child, JoinHandle<()>) {
    println!("running: {:?}", cmd);

    // Capture the standard error coming from these programs, and write it out
    // with cargo:warning= prefixes. Note that this is a bit wonky to avoid
    // requiring the output to be UTF-8, we instead just ship bytes from one
    // location to another.
    match cmd.stderr(Stdio::piped()).spawn() {
        Ok(mut child) => {
            let stderr = BufReader::new(child.stderr.take().unwrap());
            let print = thread::spawn(move || {
                for line in stderr.split(b'\n').filter_map(|l| l.ok()) {
                    print!("cargo:warning=");
                    std::io::stdout().write_all(&line).unwrap();
                    println!("");
                }
            });
            (child, print)
        }
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            fail(&format!("failed to execute command: {}\nIs `{}` \
                           not installed?",
                          e,
                          program));
        }
        Err(e) => fail(&format!("failed to execute command: {}", e)),
    }
}

fn fail(s: &str) -> ! {
    println!("\n\n{}\n\n", s);
    panic!()
}
