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

use enumflags::BitFlags;

use super::{BufferViewFormat, Validate, DeviceCapabilities, Marker};

/// Handle for buffer objects each of which represents a continuous region on a host/device memory.
///
/// Buffers are allocated from `Heap` and must not outlive the `Heap` they were created from.
pub trait Buffer: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any + Marker {}

/// Handle for buffer view objects.
///
/// Holds an implicit reference to the originating `Buffer`.
///
/// TODO: remove
pub trait BufferView: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {}

#[derive(Debug, Clone, Copy)]
pub struct BufferDescription {
    pub usage: BitFlags<BufferUsageFlags>,
    pub size: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct BufferViewDescription<'a, TBuffer: Buffer> {
    // TODO: how do we support this in Metal?
    pub buffer: &'a TBuffer,
    pub format: BufferViewFormat,
    pub offset: usize,
    pub range: usize,
}

// prevent `InnerXXX` from being exported
mod flags {
    #[derive(EnumFlags, Copy, Clone, Debug, Hash)]
    #[repr(u32)]
    pub enum BufferUsageFlags {
        TransferSource = 0b000000001,
        TransferDestination = 0b000000010,
        UniformTexelBuffer = 0b000000100,
        StorageTexelBuffer = 0b000001000,
        UniformBuffer = 0b000010000,
        StorageBuffer = 0b000100000,
        IndexBuffer = 0b001000000,
        VertexBuffer = 0b010000000,
        IndirectBuffer = 0b100000000,
    }
}

pub use self::flags::BufferUsageFlags;

/// Validation errors for [`BufferDescription`](struct.BufferDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum BufferDescriptionValidationError {
    // TODO
}

impl Validate for BufferDescription {
    type Error = BufferDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
        where T: FnMut(Self::Error) -> ()
    {
        // TODO
    }
}

/// Validation errors for [`BufferViewDescription`](struct.BufferViewDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum BufferViewDescriptionValidationError {
    // TODO
}

impl<'a, TBuffer: Buffer> Validate for BufferViewDescription<'a, TBuffer> {
    type Error = BufferViewDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
        where T: FnMut(Self::Error) -> ()
    {
        // TODO
    }
}
