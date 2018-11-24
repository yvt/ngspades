// Copyright 2016 metal-rs developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use cocoa::foundation::NSUInteger;
use objc::runtime::Class;
use objc_foundation::{INSString, NSString};
use std::mem::transmute_copy;

use super::{id, NSObjectProtocol, NSObjectPrototype};

use buffer::MTLBuffer;
use device::MTLDevice;
use texture::{MTLTexture, MTLTextureDescriptor};

#[repr(u64)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MTLPurgeableState {
    KeepCurrent = 1,
    NonVolatile = 2,
    Volatile = 3,
    Empty = 4,
}

#[repr(u64)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MTLCPUCacheMode {
    DefaultCache = 0,
    WriteCombined = 1,
}

#[repr(u64)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MTLStorageMode {
    Shared = 0,
    Managed = 1,
    Private = 2,
}

pub const MTLResourceCPUCacheModeShift: NSUInteger = 0;
pub const MTLResourceCPUCacheModeMask: NSUInteger = (0xf << MTLResourceCPUCacheModeShift);
pub const MTLResourceStorageModeShift: NSUInteger = 4;
pub const MTLResourceStorageModeMask: NSUInteger = (0xf << MTLResourceStorageModeShift);
pub const MTLResourceHazardTrackingModeShift: NSUInteger = 8;
pub const MTLResourceHazardTrackingModeMask: NSUInteger = (0x1 << MTLResourceStorageModeShift);

bitflags! {
    #[allow(non_upper_case_globals)]
    pub flags MTLResourceOptions: NSUInteger {
        const MTLResourceCPUCacheModeDefaultCache  = (MTLCPUCacheMode::DefaultCache as NSUInteger) << MTLResourceCPUCacheModeShift,
        const MTLResourceCPUCacheModeWriteCombined = (MTLCPUCacheMode::WriteCombined as NSUInteger) << MTLResourceCPUCacheModeShift,

        const MTLResourceStorageModeShared  = (MTLStorageMode::Shared as NSUInteger)  << MTLResourceStorageModeShift,
        const MTLResourceStorageModeManaged = (MTLStorageMode::Managed as NSUInteger) << MTLResourceStorageModeShift,
        const MTLResourceStorageModePrivate = (MTLStorageMode::Private as NSUInteger) << MTLResourceStorageModeShift,

        const MTLResourceHazardTrackingModeUntracked = 1 << MTLResourceHazardTrackingModeShift,

        // Deprecated spellings
        const MTLResourceOptionCPUCacheModeDefault       = MTLResourceCPUCacheModeDefaultCache.bits,
        const MTLResourceOptionCPUCacheModeWriteCombined = MTLResourceCPUCacheModeWriteCombined.bits,
    }
}

pub enum MTLResourcePrototype {}
pub type MTLResource = id<(MTLResourcePrototype, (NSObjectPrototype, ()))>;

impl<'a> MTLResource {
    pub fn label(&'a self) -> &'a str {
        unsafe {
            let label: &'a NSString = msg_send![self.0, label];
            label.as_str()
        }
    }

    pub fn set_label(&self, label: &str) {
        unsafe {
            let nslabel = NSString::from_str(label);
            msg_send![self.0, setLabel:transmute_copy::<_, *const ()>(&nslabel)]
        }
    }

    pub fn device(&self) -> MTLDevice {
        unsafe { msg_send![self.0, device] }
    }

    pub fn cpu_cache_mode(&self) -> MTLCPUCacheMode {
        unsafe { msg_send![self.0, cpuCacheMode] }
    }

    pub fn storage_mode(&self) -> MTLStorageMode {
        unsafe { msg_send![self.0, storageMode] }
    }

    pub fn set_purgeable_state(&self, state: MTLPurgeableState) -> MTLPurgeableState {
        unsafe { msg_send![self.0, setPurgeableState: state] }
    }

    pub fn make_aliasable(&self) {
        unsafe { msg_send![self.0, makeAliasable] }
    }
}

impl NSObjectProtocol for MTLResource {
    unsafe fn class() -> &'static Class {
        Class::get("MTLResource").unwrap()
    }
}

pub enum MTLHeapDescriptorPrototype {}
pub type MTLHeapDescriptor = id<(MTLHeapDescriptorPrototype, (NSObjectPrototype, ()))>;

impl<'a> MTLHeapDescriptor {
    pub fn new() -> Self {
        unsafe { msg_send![Self::class(), new] }
    }

    pub fn alloc() -> Self {
        unsafe { msg_send![Self::class(), alloc] }
    }

    pub fn init(&self) -> Self {
        unsafe { msg_send![self, init] }
    }

    pub fn cpu_cache_mode(&self) -> MTLCPUCacheMode {
        unsafe { msg_send![self.0, cpuCacheMode] }
    }

    pub fn set_cpu_cache_mode(&self, mode: MTLCPUCacheMode) {
        unsafe { msg_send![self.0, setCpuCacheMode: mode] }
    }

    pub fn storage_mode(&self) -> MTLStorageMode {
        unsafe { msg_send![self.0, storageMode] }
    }

    pub fn set_storage_mode(&self, mode: MTLStorageMode) {
        unsafe { msg_send![self.0, setStorageMode: mode] }
    }

    pub fn size(&self) -> u64 {
        unsafe { msg_send![self.0, size] }
    }

    pub fn set_size(&self, mode: u64) {
        unsafe { msg_send![self.0, setSize: mode] }
    }
}

impl NSObjectProtocol for MTLHeapDescriptor {
    unsafe fn class() -> &'static Class {
        Class::get("MTLHeapDescriptor").unwrap()
    }
}

pub enum MTLHeapPrototype {}
pub type MTLHeap = id<(MTLHeapPrototype, (NSObjectPrototype, ()))>;

impl<'a> MTLHeap {
    pub fn label(&'a self) -> &'a str {
        unsafe {
            let label: &'a NSString = msg_send![self.0, label];
            label.as_str()
        }
    }

    pub fn set_label(&self, label: &str) {
        unsafe {
            let nslabel = NSString::from_str(label);
            msg_send![self.0, setLabel:transmute_copy::<_, *const ()>(&nslabel)]
        }
    }

    pub fn cpu_cache_mode(&self) -> MTLCPUCacheMode {
        unsafe { msg_send![self.0, cpuCacheMode] }
    }

    pub fn storage_mode(&self) -> MTLStorageMode {
        unsafe { msg_send![self.0, storageMode] }
    }

    pub fn size(&self) -> u64 {
        unsafe { msg_send![self.0, size] }
    }

    pub fn used_size(&self) -> u64 {
        unsafe { msg_send![self.0, usedSize] }
    }

    pub fn current_allocated_size(&self) -> u64 {
        unsafe { msg_send![self.0, currentAllocatedSize] }
    }

    pub fn max_available_size_with_alignment(&self, alignment: u64) -> u64 {
        unsafe { msg_send![self.0, maxAvailableSizeWithAlignment: alignment] }
    }

    pub fn set_purgeable_state(&self, state: MTLPurgeableState) -> MTLPurgeableState {
        unsafe { msg_send![self.0, setPurgeableState: state] }
    }

    pub fn new_buffer(&self, length: u64, options: MTLResourceOptions) -> MTLBuffer {
        unsafe {
            msg_send![self.0, newBufferWithLength:length
                                          options:options]
        }
    }

    pub fn new_texture(&self, descriptor: MTLTextureDescriptor) -> MTLTexture {
        unsafe { msg_send![self.0, newTextureWithDescriptor:descriptor.0] }
    }
}

impl NSObjectProtocol for MTLHeap {
    unsafe fn class() -> &'static Class {
        Class::get("MTLHeap").unwrap()
    }
}
