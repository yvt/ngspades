// Copyright 2016 metal-rs developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use objc::runtime::Class;
use objc::runtime::{YES, NO};

use super::{id, NSObjectPrototype, NSObjectProtocol};

use depthstencil::MTLCompareFunction;

#[repr(u64)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MTLSamplerMinMagFilter {
    Nearest = 0,
    Linear = 1,
}

#[repr(u64)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MTLSamplerMipFilter {
    NotMipmapped = 0,
    Nearest = 1,
    Linear = 2,
}

#[repr(u64)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MTLSamplerAddressMode {
    ClampToEdge = 0,
    MirrorClampToEdge = 1,
    Repeat = 2,
    MirrorRepeat = 3,
    ClampToZero = 4,

    /// Available in macOS 10.12+.
    ClampToBorderColor = 5,
}

/// Available in macOS 10.12+.
#[repr(u64)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MTLSamplerBorderColor {
    TransparentBlack = 0,
    OpaqueBlack = 1,
    OpaqueWhite = 2,
}

pub enum MTLSamplerDescriptorPrototype {}
pub type MTLSamplerDescriptor = id<(MTLSamplerDescriptorPrototype, (NSObjectPrototype, ()))>;

impl MTLSamplerDescriptor {
    pub fn new() -> Self {
        unsafe {
            msg_send![Self::class(), new]
        }
    }

    pub fn alloc() -> Self {
        unsafe {
            msg_send![Self::class(), alloc]
        }
    }

    pub fn init(&self) -> Self {
        unsafe {
            msg_send![self, init]
        }
    }

    pub fn set_min_filter(&self, filter: MTLSamplerMinMagFilter) {
        unsafe {
            msg_send![self.0, setMinFilter:filter]
        }
    }

    pub fn set_mag_filter(&self, filter: MTLSamplerMinMagFilter) {
        unsafe {
            msg_send![self.0, setMagFilter:filter]
        }
    }

    pub fn set_mip_filter(&self, filter: MTLSamplerMipFilter) {
        unsafe {
            msg_send![self.0, setMipFilter:filter]
        }
    }

    pub fn set_address_mode_s(&self, mode: MTLSamplerAddressMode) {
        unsafe {
            msg_send![self.0, setSAddressMode:mode]
        }
    }

    pub fn set_address_mode_t(&self, mode: MTLSamplerAddressMode) {
        unsafe {
            msg_send![self.0, setTAddressMode:mode]
        }
    }

    pub fn set_address_mode_r(&self, mode: MTLSamplerAddressMode) {
        unsafe {
            msg_send![self.0, setRAddressMode:mode]
        }
    }

    pub fn set_max_anisotropy(&self, anisotropy: u64) {
        unsafe {
            msg_send![self.0, setMaxAnisotropy:anisotropy]
        }
    }

    pub fn set_compare_function(&self, func: MTLCompareFunction) {
        unsafe {
            msg_send![self.0, setCompareFunction:func]
        }
    }

    /// Available in macOS 10.12+.
    pub fn set_border_color(&self, border_color: MTLSamplerBorderColor) {
        unsafe {
            msg_send![self.0, setBorderColor:border_color]
        }
    }

    pub fn set_lod_bias(&self, bias: f32) {
        unsafe {
            msg_send![self.0, setLodBias:bias]
        }
    }

    pub fn set_lod_min_clamp(&self, clamp: f32) {
        unsafe {
            msg_send![self.0, setLodMinClamp:clamp]
        }
    }

    pub fn set_lod_max_clamp(&self, clamp: f32) {
        unsafe {
            msg_send![self.0, setLodMaxClamp:clamp]
        }
    }

    pub fn set_normalized_coordinates(&self, normalized: bool) {
        unsafe {
            msg_send![self.0, setNormalizedCoordinates:if normalized {
                YES
            } else { NO }]
        }
    }
}

impl NSObjectProtocol for MTLSamplerDescriptor {
    unsafe fn class() -> &'static Class {
        Class::get("MTLSamplerDescriptor").unwrap()
    }
}

pub enum MTLSamplerStatePrototype {}
pub type MTLSamplerState = id<(MTLSamplerStatePrototype, (NSObjectPrototype, ()))>;

impl NSObjectProtocol for MTLSamplerState {
    unsafe fn class() -> &'static Class {
        Class::get("MTLSamplerState").unwrap()
    }
}
