//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use bitflags::bitflags;
use flags_macro::flags;
use lazy_static::lazy_static;

use zangfx_common::BinaryInteger;

bitflags! {
    /// Specifies zero or more pipeline stages.
    pub struct StageFlags: u16 {
        const IndirectDraw = 0b1;
        const VertexInput = 0b10;
        const Vertex = 0b100;
        const Fragment = 0b1000;
        const EarlyFragTests = 0b10000;
        const LateFragTests = 0b100000;
        const RenderOutput = 0b1000000;
        const Compute = 0b10000000;
        const Copy = 0b100000000;
    }
}

impl StageFlags {
    pub fn all_render() -> StageFlags {
        flags![StageFlags::{IndirectDraw | VertexInput | Vertex | Fragment |
            EarlyFragTests | LateFragTests | RenderOutput}]
    }
}

bitflags! {
    /// Specifies zero or more types of memory access.
    pub struct AccessTypeFlags: u32 {
        const IndirectDrawRead = 0b1;
        const IndexRead = 0b10;
        const VertexAttrRead = 0b100;
        const VertexUniformRead = 0b1000;
        const VertexRead = 0b10000;
        const VertexWrite = 0b100000;
        const FragmentUniformRead = 0b1000000;
        const FragmentRead = 0b10000000;
        const FragmentWrite = 0b100000000;
        const ColorRead = 0b1000000000;
        const ColorWrite = 0b10000000000;
        const DsRead = 0b100000000000;
        const DsWrite = 0b1000000000000;
        const CopyRead = 0b10000000000000;
        const CopyWrite = 0b100000000000000;
        const ComputeUniformRead = 0b1000000000000000;
        const ComputeRead = 0b10000000000000000;
        const ComputeWrite = 0b100000000000000000;
    }
}

lazy_static! {
    static ref ACCESS_TO_STAGES: [StageFlags; 18] = [
        // IndirectDrawRead
        flags![StageFlags::{IndirectDraw}],
        // IndexRead
        flags![StageFlags::{VertexInput}],
        // VertexAttrRead
        flags![StageFlags::{VertexInput}],
        // VertexUniformRead
        flags![StageFlags::{Vertex}],
        // VertexRead
        flags![StageFlags::{Vertex}],
        // VertexWrite
        flags![StageFlags::{Vertex}],
        // FragmentUniformRead
        flags![StageFlags::{Fragment}],
        // FragmentRead
        flags![StageFlags::{Fragment}],
        // FragmentWrite
        flags![StageFlags::{Fragment}],
        // ColorRead
        flags![StageFlags::{RenderOutput}],
        // ColorWrite
        flags![StageFlags::{RenderOutput}],
        // DsRead
        flags![StageFlags::{EarlyFragTests | LateFragTests}],
        // DsWrite
        flags![StageFlags::{EarlyFragTests | LateFragTests}],
        // CopyRead
        flags![StageFlags::{Copy}],
        // CopyWrite
        flags![StageFlags::{Copy}],
        // ComputeUniformRead
        flags![StageFlags::{Compute}],
        // ComputeRead
        flags![StageFlags::{Compute}],
        // ComputeWrite
        flags![StageFlags::{Compute}],
    ];
}

impl AccessTypeFlags {
    /// Return a set of pipeline stages supporting at least one of given access
    /// types.
    pub fn supported_stages(&self) -> StageFlags {
        (*self & Self::all())
            .bits()
            .one_digits()
            .fold(StageFlags::empty(), |x, i| {
                x | unsafe { *ACCESS_TO_STAGES.get_unchecked(i as usize) }
            })
    }
}

bitflags! {
    /// Specifies a color channel.
    pub struct ColorChannelFlags: u8 {
        const Red = 0b0001;
        const Green = 0b0010;
        const Blue = 0b0100;
        const Alpha = 0b1000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_type_supported_stages() {
        assert_eq!(
            AccessTypeFlags::FragmentUniformRead.supported_stages(),
            flags![StageFlags::{Fragment}]
        );
        assert_eq!(
            AccessTypeFlags::CopyRead.supported_stages(),
            flags![StageFlags::{Copy}]
        );
        assert_eq!(
            AccessTypeFlags::CopyWrite.supported_stages(),
            flags![StageFlags::{Copy}]
        );
        assert_eq!(
            AccessTypeFlags::ComputeRead.supported_stages(),
            flags![StageFlags::{Compute}]
        );
        assert_eq!(
            AccessTypeFlags::ComputeWrite.supported_stages(),
            flags![StageFlags::{Compute}]
        );
        assert_eq!(
            flags![AccessTypeFlags::{FragmentUniformRead}].supported_stages(),
            flags![StageFlags::{Fragment}]
        );
        assert_eq!(
            flags![AccessTypeFlags::{VertexRead | FragmentWrite}].supported_stages(),
            flags![StageFlags::{Vertex | Fragment}]
        );
    }
}
