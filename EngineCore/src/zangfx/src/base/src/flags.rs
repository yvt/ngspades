//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use lazy_static::lazy_static;
use ngsenumflags::BitFlags;

use crate::common::BinaryInteger;

/// Specifies a pipeline stage.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum Stage {
    IndirectDraw = 0b1,
    VertexInput = 0b10,
    Vertex = 0b100,
    Fragment = 0b1000,
    EarlyFragTests = 0b10000,
    LateFragTests = 0b100000,
    RenderOutput = 0b1000000,
    Compute = 0b10000000,
    Copy = 0b100000000,
}

impl Stage {
    pub fn all() -> StageFlags {
        StageFlags::all()
    }

    pub fn all_render() -> StageFlags {
        flags![Stage::{IndirectDraw | VertexInput | Vertex | Fragment | EarlyFragTests |
            LateFragTests | RenderOutput}]
    }
}

/// Specifies zero or more pipeline stages.
pub type StageFlags = BitFlags<Stage>;

/// Specifies a type of memory access.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum AccessType {
    IndirectDrawRead = 0b1,
    IndexRead = 0b10,
    VertexAttrRead = 0b100,
    VertexUniformRead = 0b1000,
    VertexRead = 0b10000,
    VertexWrite = 0b100000,
    FragmentUniformRead = 0b1000000,
    FragmentRead = 0b10000000,
    FragmentWrite = 0b100000000,
    ColorRead = 0b1000000000,
    ColorWrite = 0b10000000000,
    DsRead = 0b100000000000,
    DsWrite = 0b1000000000000,
    CopyRead = 0b10000000000000,
    CopyWrite = 0b100000000000000,
    ComputeUniformRead = 0b1000000000000000,
    ComputeRead = 0b10000000000000000,
    ComputeWrite = 0b100000000000000000,
}

lazy_static! {
    static ref ACCESS_TO_STAGES: [StageFlags; 18] = [
        // IndirectDrawRead
        flags![Stage::{IndirectDraw}],
        // IndexRead
        flags![Stage::{VertexInput}],
        // VertexAttrRead
        flags![Stage::{VertexInput}],
        // VertexUniformRead
        flags![Stage::{Vertex}],
        // VertexRead
        flags![Stage::{Vertex}],
        // VertexWrite
        flags![Stage::{Vertex}],
        // FragmentUniformRead
        flags![Stage::{Fragment}],
        // FragmentRead
        flags![Stage::{Fragment}],
        // FragmentWrite
        flags![Stage::{Fragment}],
        // ColorRead
        flags![Stage::{RenderOutput}],
        // ColorWrite
        flags![Stage::{RenderOutput}],
        // DsRead
        flags![Stage::{EarlyFragTests | LateFragTests}],
        // DsWrite
        flags![Stage::{EarlyFragTests | LateFragTests}],
        // CopyRead
        flags![Stage::{Copy}],
        // CopyWrite
        flags![Stage::{Copy}],
        // ComputeUniformRead
        flags![Stage::{Compute}],
        // ComputeRead
        flags![Stage::{Compute}],
        // ComputeWrite
        flags![Stage::{Compute}],
    ];
}

impl AccessType {
    pub fn supported_stages(&self) -> StageFlags {
        let i = (*self as u32).trailing_zeros();
        unsafe { *ACCESS_TO_STAGES.get_unchecked(i as usize) }
    }

    pub fn union_supported_stages(access: AccessTypeFlags) -> StageFlags {
        access.bits().one_digits().fold(flags![Stage::{}], |x, i| {
            x | unsafe { *ACCESS_TO_STAGES.get_unchecked(i as usize) }
        })
    }
}

/// Specifies zero or more types of memory access.
pub type AccessTypeFlags = BitFlags<AccessType>;

/// Specifies a color channel.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash)]
#[repr(u32)]
pub enum ColorChannel {
    Red = 0b0001,
    Green = 0b0010,
    Blue = 0b0100,
    Alpha = 0b1000,
}

/// Specifies zero or more color channels.
pub type ColorChannelFlags = BitFlags<ColorChannel>;

impl ColorChannel {
    /// Return a value specifying all channels.
    pub fn all() -> ColorChannelFlags {
        flags![ColorChannel::{Red | Green | Blue | Alpha}]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_type_supported_stages() {
        assert_eq!(
            AccessType::FragmentUniformRead.supported_stages(),
            flags![Stage::{Fragment}]
        );
        assert_eq!(
            AccessType::CopyRead.supported_stages(),
            flags![Stage::{Copy}]
        );
        assert_eq!(
            AccessType::CopyWrite.supported_stages(),
            flags![Stage::{Copy}]
        );
        assert_eq!(
            AccessType::ComputeRead.supported_stages(),
            flags![Stage::{Compute}]
        );
        assert_eq!(
            AccessType::ComputeWrite.supported_stages(),
            flags![Stage::{Compute}]
        );
    }

    #[test]
    fn access_type_flags_supported_stages() {
        assert_eq!(
            AccessType::union_supported_stages(flags![AccessType::{FragmentUniformRead}]),
            flags![Stage::{Fragment}]
        );
        assert_eq!(
            AccessType::union_supported_stages(flags![AccessType::{VertexRead | FragmentWrite}]),
            flags![Stage::{Vertex | Fragment}]
        );
    }
}
