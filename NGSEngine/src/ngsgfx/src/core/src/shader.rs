//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::clone::Clone;
use std::hash::Hash;
use std::fmt::Debug;
use std::cmp::{Eq, PartialEq};
use std::any::Any;

/// Handle for shader modules.
pub trait ShaderModule: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {}

#[derive(Debug, Clone, Copy)]
pub struct ShaderModuleDescription<'a> {
    pub spirv_code: &'a [u32],
}

// prevent `InnerXXX` from being exported
mod flags {
    #[derive(EnumFlags, Copy, Clone, Debug, Hash)]
    #[repr(u8)]
    pub enum ShaderStageFlags {
        Vertex = 0b001,
        Fragment = 0b010,
        Compute = 0b100,
    }
}

pub use self::flags::ShaderStageFlags;
