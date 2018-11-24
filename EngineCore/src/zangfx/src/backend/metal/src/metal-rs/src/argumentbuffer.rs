//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

use cocoa::foundation::{NSRange, NSUInteger};
use objc::runtime::Class;
use objc_foundation::{INSString, NSString};
use std::mem::transmute_copy;

use super::{id, NSObjectProtocol, NSObjectPrototype};

use libc;

use argument::{MTLArgumentAccess, MTLDataType};
use buffer::MTLBuffer;
use sampler::MTLSamplerState;
use texture::{MTLTexture, MTLTextureType};

pub enum MTLArgumentEncoderPrototype {}
pub type MTLArgumentEncoder = id<(MTLArgumentEncoderPrototype, (NSObjectPrototype, ()))>;

impl<'a> MTLArgumentEncoder {
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

    pub fn set_argument_buffer(&self, buffer: MTLBuffer, offset: NSUInteger) {
        unsafe {
            msg_send![self.0, setArgumentBuffer:buffer.0
                                         offset:offset]
        }
    }

    pub fn encoded_length(&self) -> NSUInteger {
        unsafe { msg_send![self.0, encodedLength] }
    }

    pub fn alignment(&self) -> NSUInteger {
        unsafe { msg_send![self.0, alignment] }
    }

    pub fn set_buffer(&self, buffer: MTLBuffer, offset: NSUInteger, index: NSUInteger) {
        unsafe {
            msg_send![self.0, setBuffer:buffer
                                 offset:offset
                                atIndex:index]
        }
    }

    pub fn set_buffers(&self, buffers: &[MTLBuffer], offsets: &[NSUInteger], index: NSUInteger) {
        let range = NSRange {
            location: index,
            length: buffers.len() as NSUInteger,
        };
        unsafe {
            msg_send![self.0, setBuffers:buffers.as_ptr()
                                 offsets:offsets.as_ptr()
                               withRange:range]
        }
    }

    pub fn set_texture(&self, texture: MTLTexture, index: NSUInteger) {
        unsafe {
            msg_send![self.0, setTexture:texture
                                 atIndex:index]
        }
    }

    pub fn set_textures(&self, textures: &[MTLTexture], index: NSUInteger) {
        let range = NSRange {
            location: index,
            length: textures.len() as NSUInteger,
        };
        unsafe {
            msg_send![self.0, setTextures:textures.as_ptr()
                                withRange:range]
        }
    }

    pub fn set_sampler_state(&self, sampler: MTLSamplerState, index: NSUInteger) {
        unsafe {
            msg_send![self.0, setSamplerState:sampler
                                      atIndex:index]
        }
    }

    pub fn set_sampler_states(&self, samplers: &[MTLSamplerState], index: NSUInteger) {
        let range = NSRange {
            location: index,
            length: samplers.len() as NSUInteger,
        };
        unsafe {
            msg_send![self.0, setSamplerStates:samplers.as_ptr()
                                     withRange:range]
        }
    }

    pub fn constant_data(&self, index: NSUInteger) -> *mut libc::c_void {
        unsafe { msg_send![self.0, constantDataAtIndex: index] }
    }

    pub fn new_argument_encoder_for_buffer(&self, index: NSUInteger) -> MTLArgumentEncoder {
        unsafe { msg_send![self.0, newArgumentEncoderForBufferAtIndex: index] }
    }
}

impl NSObjectProtocol for MTLArgumentEncoder {
    unsafe fn class() -> &'static Class {
        Class::get("MTLArgumentEncoder").unwrap()
    }
}

pub enum MTLArgumentDescriptorPrototype {}
pub type MTLArgumentDescriptor = id<(MTLArgumentDescriptorPrototype, (NSObjectPrototype, ()))>;

impl MTLArgumentDescriptor {
    pub fn new() -> Self {
        unsafe { msg_send![Self::class(), argumentDescriptor] }
    }

    pub fn data_type(&self) -> MTLDataType {
        unsafe { msg_send![self.0, dataType] }
    }

    pub fn set_data_type(&self, data_type: MTLDataType) {
        unsafe { msg_send![self.0, setDataType: data_type] }
    }

    pub fn index(&self) -> NSUInteger {
        unsafe { msg_send![self.0, index] }
    }

    pub fn set_index(&self, index: NSUInteger) {
        unsafe { msg_send![self.0, setIndex: index] }
    }

    pub fn access(&self) -> MTLArgumentAccess {
        unsafe { msg_send![self.0, access] }
    }

    pub fn set_access(&self, access: MTLArgumentAccess) {
        unsafe { msg_send![self.0, setAccess: access] }
    }

    pub fn array_length(&self) -> NSUInteger {
        unsafe { msg_send![self.0, arrayLength] }
    }

    pub fn set_array_length(&self, array_length: NSUInteger) {
        unsafe { msg_send![self.0, setArrayLength: array_length] }
    }

    pub fn constant_block_alignment(&self) -> NSUInteger {
        unsafe { msg_send![self.0, constantBlockAlignment] }
    }

    pub fn set_constant_block_alignment(&self, constant_block_alignment: NSUInteger) {
        unsafe { msg_send![self.0, setConstantBlockAlignment: constant_block_alignment] }
    }

    pub fn texture_type(&self) -> MTLTextureType {
        unsafe { msg_send![self.0, textureType] }
    }

    pub fn set_texture_type(&self, texture_type: MTLTextureType) {
        unsafe { msg_send![self.0, setTextureType: texture_type] }
    }
}

impl NSObjectProtocol for MTLArgumentDescriptor {
    unsafe fn class() -> &'static Class {
        Class::get("MTLArgumentDescriptor").unwrap()
    }
}
