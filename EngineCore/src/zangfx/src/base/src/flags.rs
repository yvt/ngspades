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
        const INDIRECT_DRAW = 0b1;
        const VERTEX_INPUT = 0b10;
        const VERTEX = 0b100;
        const FRAGMENT = 0b1000;
        const EARLY_FRAG_TESTS = 0b10000;
        const LATE_FRAG_TESTS = 0b100000;
        const RENDER_OUTPUT = 0b1000000;
        const COMPUTE = 0b10000000;
        const COPY = 0b100000000;
    }
}

impl StageFlags {
    pub fn all_render() -> StageFlags {
        flags![StageFlags::{INDIRECT_DRAW | VERTEX_INPUT | VERTEX | FRAGMENT |
            EARLY_FRAG_TESTS | LATE_FRAG_TESTS | RENDER_OUTPUT}]
    }
}

bitflags! {
    /// Specifies zero or more types of memory access.
    pub struct AccessTypeFlags: u32 {
        const INDIRECT_DRAW_READ = 0b1;
        const INDEX_READ = 0b10;
        const VERTEX_ATTR_READ = 0b100;
        const VERTEX_UNIFORM_READ = 0b1000;
        const VERTEX_READ = 0b10000;
        const VERTEX_WRITE = 0b100000;
        const FRAGMENT_UNIFORM_READ = 0b1000000;
        const FRAGMENT_READ = 0b10000000;
        const FRAGMENT_WRITE = 0b100000000;
        const COLOR_READ = 0b1000000000;
        const COLOR_WRITE = 0b10000000000;
        const DS_READ = 0b100000000000;
        const DS_WRITE = 0b1000000000000;
        const COPY_READ = 0b10000000000000;
        const COPY_WRITE = 0b100000000000000;
        const COMPUTE_UNIFORM_READ = 0b1000000000000000;
        const COMPUTE_READ = 0b10000000000000000;
        const COMPUTE_WRITE = 0b100000000000000000;
    }
}

lazy_static! {
    static ref ACCESS_TO_STAGES: [StageFlags; 18] = [
        // IndirectDrawRead
        flags![StageFlags::{INDIRECT_DRAW}],
        // IndexRead
        flags![StageFlags::{VERTEX_INPUT}],
        // VertexAttrRead
        flags![StageFlags::{VERTEX_INPUT}],
        // VertexUniformRead
        flags![StageFlags::{VERTEX}],
        // VertexRead
        flags![StageFlags::{VERTEX}],
        // VertexWrite
        flags![StageFlags::{VERTEX}],
        // FragmentUniformRead
        flags![StageFlags::{FRAGMENT}],
        // FragmentRead
        flags![StageFlags::{FRAGMENT}],
        // FragmentWrite
        flags![StageFlags::{FRAGMENT}],
        // ColorRead
        flags![StageFlags::{RENDER_OUTPUT}],
        // ColorWrite
        flags![StageFlags::{RENDER_OUTPUT}],
        // DsRead
        flags![StageFlags::{EARLY_FRAG_TESTS | LATE_FRAG_TESTS}],
        // DsWrite
        flags![StageFlags::{EARLY_FRAG_TESTS | LATE_FRAG_TESTS}],
        // CopyRead
        flags![StageFlags::{COPY}],
        // CopyWrite
        flags![StageFlags::{COPY}],
        // ComputeUniformRead
        flags![StageFlags::{COMPUTE}],
        // ComputeRead
        flags![StageFlags::{COMPUTE}],
        // ComputeWrite
        flags![StageFlags::{COMPUTE}],
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
        const RED = 0b0001;
        const GREEN = 0b0010;
        const BLUE = 0b0100;
        const ALPHA = 0b1000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_type_supported_stages() {
        assert_eq!(
            AccessTypeFlags::FRAGMENT_UNIFORM_READ.supported_stages(),
            flags![StageFlags::{FRAGMENT}]
        );
        assert_eq!(
            AccessTypeFlags::COPY_READ.supported_stages(),
            flags![StageFlags::{COPY}]
        );
        assert_eq!(
            AccessTypeFlags::COPY_WRITE.supported_stages(),
            flags![StageFlags::{COPY}]
        );
        assert_eq!(
            AccessTypeFlags::COMPUTE_READ.supported_stages(),
            flags![StageFlags::{COMPUTE}]
        );
        assert_eq!(
            AccessTypeFlags::COMPUTE_WRITE.supported_stages(),
            flags![StageFlags::{COMPUTE}]
        );
        assert_eq!(
            flags![AccessTypeFlags::{FRAGMENT_UNIFORM_READ}].supported_stages(),
            flags![StageFlags::{FRAGMENT}]
        );
        assert_eq!(
            flags![AccessTypeFlags::{VERTEX_READ | FRAGMENT_WRITE}].supported_stages(),
            flags![StageFlags::{VERTEX | FRAGMENT}]
        );
    }
}
