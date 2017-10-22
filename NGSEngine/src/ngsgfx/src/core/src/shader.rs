//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines types related to shader modules.
use std::clone::Clone;
use std::hash::Hash;
use std::fmt::Debug;
use std::cmp::{Eq, PartialEq};
use std::any::Any;

use ngsenumflags::BitFlags;

use {Marker, Validate, DeviceCapabilities};

/// Shader module handle.
pub trait ShaderModule
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any + Marker {
}

#[derive(Debug, Clone, Copy)]
pub struct ShaderModuleDescription<'a> {
    /// SPIR-V code.
    ///
    /// See Vulkan 1.0 Specification Appendix A: "Vulkan Environment for SPIR-V"
    /// for the requirements.
    pub spirv_code: &'a [u32],
}

#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum ShaderStage {
    Vertex = 0b001,
    Fragment = 0b010,
    Compute = 0b100,
}

pub type ShaderStageFlags = BitFlags<ShaderStage>;

/// Validation errors for [`ShaderModuleDescription`](struct.ShaderModuleDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ShaderModuleDescriptionValidationError {
    /// An invalid SPIR-V code was detected.
    ///
    /// Implementation note: the current implemention only checks the magic number for a
    /// SPIR-V module.
    InvalidSpirvCode,
}

impl<'a> Validate for ShaderModuleDescription<'a> {
    type Error = ShaderModuleDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        if self.spirv_code.len() == 0 ||
            (self.spirv_code[0] != 0x07230203 && self.spirv_code[0] != 0x03022307)
        {
            callback(ShaderModuleDescriptionValidationError::InvalidSpirvCode);
        }
    }
}
