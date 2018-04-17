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

use ngsenumflags::BitFlags;

use {Validate, DeviceCapabilities, Marker, DeviceSize, StorageMode};

/// Handle for buffer objects each of which represents a continuous region on a host/device memory.
///
/// Buffers are allocated from `Heap` and must not outlive the `Heap` they were created from.
pub trait Buffer
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any + Marker {
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferDescription {
    pub usage: BufferUsageFlags,
    pub size: DeviceSize,

    /// Specifies the memory location of the buffer.
    ///
    /// Only `Shared` or `Private` can be specified.
    pub storage_mode: StorageMode,
}

#[derive(NgsEnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum BufferUsage {
    TransferSource = 0b0000001,
    TransferDestination = 0b0000010,
    UniformBuffer = 0b0000100,
    StorageBuffer = 0b0001000,
    IndexBuffer = 0b0010000,
    VertexBuffer = 0b0100000,
    IndirectBuffer = 0b1000000,
}

pub type BufferUsageFlags = BitFlags<BufferUsage>;

/// Validation errors for [`BufferDescription`](struct.BufferDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum BufferDescriptionValidationError {
    /// `size` is zero.
    ZeroSize,
    /// `storage_mode` is `Memoryless`.
    StorageModeMemoryless,
}

impl Validate for BufferDescription {
    type Error = BufferDescriptionValidationError;

    fn validate<T>(&self, _: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        if self.size == 0 {
            callback(BufferDescriptionValidationError::ZeroSize);
        }
        if self.storage_mode == StorageMode::Memoryless {
            callback(BufferDescriptionValidationError::StorageModeMemoryless);
        }
    }
}