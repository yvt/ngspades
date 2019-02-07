//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for shader library objects, and other relevant types.
use bitflags::bitflags;

use crate::{Object, Result};

define_handle! {
    /// Shader library handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    LibraryRef
}

/// The builder object for shader libraries.
pub type LibraryBuilderRef = Box<dyn LibraryBuilder>;

/// Trait for building shader libraries.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device) {
///     let image = device.build_library()
///         .spirv_code(&[])
///         .build()
///         .expect_err("Succeeded to create a shader library with an invalid \
///                      SPIR-V code.");
///     # }
///
pub trait LibraryBuilder: Object {
    /// Set the SPIR-V code.
    ///
    /// See Vulkan 1.0 Specification Appendix A: "Vulkan Environment for SPIR-V"
    /// for the requirements.
    fn spirv_code(&mut self, v: &[u32]) -> &mut dyn LibraryBuilder;

    /// Build an `LibraryRef`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<LibraryRef>;
}

bitflags! {
    pub struct ShaderStageFlags: u8 {
        const VERTEX = 0b001;
        const FRAGMENT = 0b010;
        const COMPUTE = 0b100;
    }
}
