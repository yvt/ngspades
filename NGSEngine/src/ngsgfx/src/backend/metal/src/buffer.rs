//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;
use metal::NSObjectProtocol;

use std::ops::Deref;

use {OCPtr, RefEqArc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Buffer {
    data: RefEqArc<BufferData>,
}

impl core::Buffer for Buffer {}

#[derive(Debug)]
struct BufferData {
    metal_buffer: OCPtr<metal::MTLBuffer>,
    size: usize,
}

unsafe impl Send for BufferData {}
unsafe impl Sync for BufferData {} // no interior mutability

impl Buffer {
    pub(crate) fn new(device: metal::MTLDevice,
                      storage: metal::MTLStorageMode,
                      desc: &core::BufferDescription)
                      -> core::Result<Self> {
        let needs_texel_buffer = !(desc.usage &
                                   (core::BufferUsageFlags::UniformTexelBuffer |
                                    core::BufferUsageFlags::StorageTexelBuffer))
                                          .is_empty();
        if needs_texel_buffer {
            unimplemented!();
        }
        /*
        let texel_buffer_mode =
            if !needs_texel_buffer {
                TexelBufferMode::Unsupported
            } else if desc.size <= BUFFER_VIEW_TEXTURE_MAX_PITCH {
                TexelBufferMode::SingleRow
            } else {
                TexelBufferMode::MultiRow
            }; */
        let options: metal::MTLResourceOptions = match storage {
            metal::MTLStorageMode::Private => metal::MTLResourceStorageModePrivate,
            metal::MTLStorageMode::Shared => metal::MTLResourceStorageModeShared,
            metal::MTLStorageMode::Managed => metal::MTLResourceStorageModeManaged,
        };
        let metal_buffer = unsafe { OCPtr::from_raw(device.new_buffer(desc.size as u64, options)) }
            .ok_or(core::GenericError::OutOfDeviceMemory)?;
        let data = BufferData {
            metal_buffer: metal_buffer,
            size: desc.size,
        };
        Ok(Self { data: RefEqArc::new(data) })
    }

    pub(crate) unsafe fn contents(&self) -> *mut ::std::os::raw::c_void {
        ::std::mem::transmute(self.data.metal_buffer.contents())
    }

    pub(crate) fn len(&self) -> usize {
        self.data.size
    }

    pub(crate) fn metal_buffer(&self) -> &metal::MTLBuffer {
        self.data.metal_buffer.deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BufferView {
    data: RefEqArc<BufferViewData>,
}

impl core::BufferView for BufferView {}

#[derive(Debug)]
struct BufferViewData {
    metal_texture: OCPtr<metal::MTLTexture>,
}

unsafe impl Send for BufferViewData {}
unsafe impl Sync for BufferViewData {} // no interior mutability

pub const BUFFER_VIEW_TEXTURE_WIDTH: u32 = 8192;
pub const BUFFER_VIEW_TEXTURE_MAX_PITCH: u32 = 8192 * 16; // 16 = the largest pixel format's size

pub enum TexelBufferMode {
    Unsupported,
    SingleRow,
    MultiRow,
}

impl BufferView {
    pub(crate) fn metal_texture(&self) -> &metal::MTLTexture {
        self.data.metal_texture.deref()
    }
}
