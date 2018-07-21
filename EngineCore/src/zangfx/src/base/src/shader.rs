//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for shader library objects, and other relevant types.
use ngsenumflags::BitFlags;

use crate::{Object, Result};

define_handle! {
    /// Shader library handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    Library
}

/// Trait for building shader libraries.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::shader::LibraryBuilder;
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
    fn spirv_code(&mut self, v: &[u32]) -> &mut LibraryBuilder;

    /// Build an `Library`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<Library>;
}

#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum ShaderStage {
    Vertex = 0b001,
    Fragment = 0b010,
    Compute = 0b100,
}

pub type ShaderStageFlags = BitFlags<ShaderStage>;
