//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ngsenumflags::BitFlags;

/// Specifies a pipeline stage.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum Stage {
    Top = 0b00000000000001,
    IndirectDraw = 0b00000000000010,
    VertexInput = 0b00000000000100,
    Vertex = 0b00000000001000,
    Fragment = 0b00000000010000,
    EarlyFragTests = 0b00000000100000,
    LateFragTests = 0b00000001000000,
    RenderOutput = 0b00000010000000,
    Compute = 0b00000100000000,
    Copy = 0b00001000000000,
    Bottom = 0b00010000000000,
    Host = 0b00100000000000,
    AllRender = 0b01000000000000,
    All = 0b10000000000000,
}

/// Specifies zero or more pipeline stages.
pub type StageFlags = BitFlags<Stage>;

/// Specifies a type of memory access.
#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum AccessType {
    IndirectDrawRead = 0b00000000000000001,
    IndexRead = 0b00000000000000010,
    VertexRead = 0b00000000000000100,
    UniformRead = 0b00000000000001000,
    SubpassRead = 0b00000000000010000,
    ShaderRead = 0b00000000000100000,
    ShaderWrite = 0b00000000001000000,
    ColorRead = 0b00000000010000000,
    ColorWrite = 0b00000000100000000,
    DsRead = 0b00000001000000000,
    DsWrite = 0b00000010000000000,
    CopyRead = 0b00000100000000000,
    CopyWrite = 0b00001000000000000,
    HostRead = 0b00010000000000000,
    HostWrite = 0b00100000000000000,
    MemoryRead = 0b01000000000000000,
    MemoryWrite = 0b10000000000000000,
}

/// Specifies zero or more types of memory access.
pub type AccessTypeFlags = BitFlags<AccessType>;
