//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[derive(EnumFlags, Copy, Clone, Debug, Hash)]
#[repr(u32)]
pub enum PipelineStageFlags {
    TopOfPipe = 0b00000000000001,
    DrawIndirect = 0b00000000000010,
    VertexInput = 0b00000000000100,
    VertexShader = 0b00000000001000,
    FragmentShader = 0b00000000010000,
    EarlyFragmentTests = 0b00000000100000,
    LateFragmentTests = 0b00000001000000,
    ColorAttachmentOutput = 0b00000010000000,
    ComputeShader = 0b00000100000000,
    Transfer = 0b00001000000000,
    BottomOfPipe = 0b00010000000000,
    Host = 0b00100000000000,
    AllGraphics = 0b01000000000000,
    AllCommands = 0b10000000000000,
}

#[derive(EnumFlags, Copy, Clone, Debug, Hash)]
#[repr(u32)]
pub enum AccessFlags {
    IndirectCommandRead = 0b00000000000000001,
    IndexRead = 0b00000000000000010,
    VertexAttributeRead = 0b00000000000000100,
    UniformRead = 0b00000000000001000,
    InputAttachmentRead = 0b00000000000010000,
    ShaderRead = 0b00000000000100000,
    ShaderWrite = 0b00000000001000000,
    ColorAttachmentRead = 0b00000000010000000,
    ColorAttachmentWrite = 0b00000000100000000,
    DepthStencilAttachmentRead = 0b00000001000000000,
    DepthStencilAttachmentWrite = 0b00000010000000000,
    TransferRead = 0b00000100000000000,
    TransferWrite = 0b00001000000000000,
    HostRead = 0b00010000000000000,
    HostWrite = 0b00100000000000000,
    MemoryRead = 0b01000000000000000,
    MemoryWrite = 0b10000000000000000,
}
