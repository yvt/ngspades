// Copyright 2016 metal-rs developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use cocoa::foundation::{NSUInteger};
use objc::runtime::{Object, Class, YES, NO};
use objc_foundation::{NSString, INSString};
use NSArray;

use super::{id, nil, NSObjectPrototype, NSObjectProtocol};

use resource::{MTLResourceOptions, MTLHeapDescriptor, MTLHeap};
use commandqueue::MTLCommandQueue;
use pipeline::{MTLRenderPipelineState, MTLRenderPipelineDescriptor,
               MTLRenderPipelineReflection, MTLComputePipelineState,
               MTLComputePipelineDescriptor};
use library::{MTLLibrary, MTLCompileOptions};
use types::{MTLSize, MTLSizeAndAlign};
use buffer::MTLBuffer;
use texture::{MTLTexture, MTLTextureDescriptor};
use sampler::{MTLSamplerState, MTLSamplerDescriptor};
use depthstencil::{MTLDepthStencilDescriptor, MTLDepthStencilState};
use argumentbuffer::{MTLArgumentDescriptor, MTLArgumentEncoder};

use libc;

use std::marker::PhantomData;
use std::ffi::CStr;
use std::path::Path;
use std::mem::transmute_copy;

#[allow(non_camel_case_types)]
#[repr(u64)]
#[derive(Copy, Clone)]
pub enum MTLFeatureSet {
    iOS_GPUFamily1_v1 = 0,
    iOS_GPUFamily2_v1 = 1,
    iOS_GPUFamily1_v2 = 2,
    iOS_GPUFamily2_v2 = 3,
    iOS_GPUFamily3_v1 = 4,
    OSX_GPUFamily1_v1 = 10000,
    OSX_GPUFamily1_v2 = 10001,
    OSX_ReadWriteTextureTier2 = 10002,
    tvOS_GPUFamily1_v1 = 30000,
    tvOS_GPUFamily1_v2 = 30001,
}

bitflags! {
    flags MTLPipelineOption: NSUInteger {
        const MTLPipelineOptionNone           = 0,
        const MTLPipelineOptionArgumentInfo   = 1 << 0,
        const MTLPipelineOptionBufferTypeInfo = 1 << 1
    }
}

#[allow(non_camel_case_types)]
#[repr(u64)]
#[derive(Copy, Clone)]
pub enum MTLArgumentBuffersTier {
    MTLArgumentBuffersTier1 = 0,
    MTLArgumentBuffersTier2 = 1,
}

#[link(name = "Metal", kind = "framework")]
extern {
    fn MTLCreateSystemDefaultDevice() -> *mut Object;
}

pub fn create_system_default_device() -> MTLDevice {
    unsafe {
        id(MTLCreateSystemDefaultDevice(), PhantomData)
    }
}

/*type MTLNewLibraryCompletionHandler = extern fn(library: id, error: id);
type MTLNewRenderPipelineStateCompletionHandler = extern fn(renderPipelineState: id, error: id);
type MTLNewRenderPipelineStateWithReflectionCompletionHandler = extern fn(renderPipelineState: id, reflection: id, error: id);
type MTLNewComputePipelineStateCompletionHandler = extern fn(computePipelineState: id, error: id);
type MTLNewComputePipelineStateWithReflectionCompletionHandler = extern fn(computePipelineState: id, reflection: id, error: id);*/


pub enum MTLDevicePrototype {}
pub type MTLDevice = id<(MTLDevicePrototype, (NSObjectPrototype, ()))>;

impl<'a> MTLDevice {
    pub fn name(&'a self) -> &'a str {
        unsafe {
            let name: &'a NSString = msg_send![self.0, name];
            name.as_str()
        }
    }

    pub fn vendor(&'a self) -> &'a str {
        unsafe {
            let name: &'a NSString = msg_send![self.0, vendorName];
            name.as_str()
        }
    }

    pub fn family_name(&'a self) -> &'a str {
        unsafe {
            let name: &'a NSString = msg_send![self.0, familyName];
            name.as_str()
        }
    }

    pub fn max_threads_per_threadgroup(&self) -> MTLSize {
        unsafe {
            msg_send![self.0, maxThreadsPerThreadgroup]
        }
    }

    pub fn recommended_max_working_set_size(&self) -> u64 {
        unsafe {
            msg_send![self.0, recommendedMaxWorkingSetSize]
        }
    }

    pub fn is_low_power(&self) -> bool {
        unsafe {
            match msg_send![self.0, isLowPower] {
                YES => true,
                NO => false,
                _ => unreachable!()
            }
        }
    }

    pub fn is_headless(&self) -> bool {
        unsafe {
            match msg_send![self.0, isHeadless] {
                YES => true,
                NO => false,
                _ => unreachable!()
            }
        }
    }

    pub fn supports_feature_set(&self, feature: MTLFeatureSet) -> bool {
        unsafe {
            match msg_send![self.0, supportsFeatureSet:feature] {
                YES => true,
                NO => false,
                _ => unreachable!()
            }
        }
    }

    pub fn supports_sample_count(&self, count: NSUInteger) -> bool {
        unsafe {
            match msg_send![self.0, supportsTextureSampleCount:count] {
                YES => true,
                NO => false,
                _ => unreachable!()
            }
        }
    }

    pub fn d24_s8_supported(&self) -> bool {
        unsafe {
            match msg_send![self.0, isDepth24Stencil8PixelFormatSupported] {
                YES => true,
                NO => false,
                _ => unreachable!()
            }
        }
    }

    pub fn argument_buffers_support(&self) -> MTLArgumentBuffersTier {
        unsafe {
            msg_send![self.0, argumentBuffersSupport]
        }
    }

    pub fn new_command_queue(&self) -> MTLCommandQueue {
        unsafe {
            msg_send![self.0, newCommandQueue]
        }
    }

    pub fn new_default_library(&self) -> MTLLibrary {
        unsafe {
            msg_send![self.0, newDefaultLibrary]
        }
    }

    pub fn new_fence(&self) -> MTLFence {
        unsafe {
            msg_send![self.0, newFence]
        }
    }

    pub fn new_library_with_source(&self, src: &str, options: MTLCompileOptions) -> Result<MTLLibrary, String> {
        use cocoa::foundation::NSString as cocoa_NSString;
        use cocoa::base::nil as cocoa_nil;

        unsafe {
            let source = cocoa_NSString::alloc(cocoa_nil).init_str(src);
            let mut err = nil;

            let library: MTLLibrary = msg_send![self.0, newLibraryWithSource:source
                                                                     options:options
                                                                       error:&mut err];

            match library.is_null() {
                false => Ok(library),
                true => {
                    let desc: id = msg_send![err.0, localizedDescription];
                    let compile_error: *const libc::c_char = msg_send![desc.0, UTF8String];
                    Err(CStr::from_ptr(compile_error).to_string_lossy().into_owned())
                }
            }
        }
    }

    pub fn new_library_with_file<P: AsRef<Path>>(&self, file: P) -> Result<MTLLibrary, String> {
        use cocoa::foundation::NSString as cocoa_NSString;
        use cocoa::base::nil as cocoa_nil;

        unsafe {
            let filename = cocoa_NSString::alloc(cocoa_nil)
                .init_str(file.as_ref().to_string_lossy().as_ref());
            let mut err = nil;

            let library: MTLLibrary = msg_send![self.0, newLibraryWithFile:filename
                                                                     error:&mut err];

            match library.is_null() {
                false => Ok(library),
                true => {
                    let desc: id = msg_send![err.0, localizedDescription];
                    let compile_error: *const libc::c_char = msg_send![desc.0, UTF8String];
                    Err(CStr::from_ptr(compile_error).to_string_lossy().into_owned())
                }
            }
        }
    }

    pub fn new_render_pipeline_state_with_reflection(&self, descriptor: MTLRenderPipelineDescriptor, reflection: *mut MTLRenderPipelineReflection) -> Result<MTLRenderPipelineState, String> {
        unsafe {
            let reflection_options = MTLPipelineOptionArgumentInfo | MTLPipelineOptionBufferTypeInfo;
            let mut err = nil;

            let pipeline_state: MTLRenderPipelineState = msg_send![self.0, newRenderPipelineStateWithDescriptor:descriptor.0
                                                                                                        options:reflection_options
                                                                                                     reflection:reflection
                                                                                                          error:&mut err];

            match pipeline_state.is_null() {
                false => Ok(pipeline_state),
                true => {
                    let desc: id = msg_send![err.0, localizedDescription];
                    let compile_error: *const libc::c_char = msg_send![desc.0, UTF8String];
                    Err(CStr::from_ptr(compile_error).to_string_lossy().into_owned())
                }
            }
        }

    }

    pub fn new_render_pipeline_state(&self, descriptor: MTLRenderPipelineDescriptor) -> Result<MTLRenderPipelineState, String> {
        unsafe {
            let mut err = nil;
            let pipeline_state: MTLRenderPipelineState = msg_send![self.0, newRenderPipelineStateWithDescriptor:descriptor.0
                                                                                                          error:&mut err];

            match pipeline_state.is_null() {
                true => {
                    let desc: id = msg_send![err.0, localizedDescription];
                    let compile_error: *const libc::c_char = msg_send![desc.0, UTF8String];
                    Err(CStr::from_ptr(compile_error).to_string_lossy().into_owned())
                }
                false => Ok(pipeline_state)
            }
        }
    }

    pub fn new_compute_pipeline_state(&self, descriptor: MTLComputePipelineDescriptor) -> Result<MTLComputePipelineState, String> {
        unsafe {
            let mut err = nil;
            let pipeline_state: MTLComputePipelineState = msg_send![self.0, newComputePipelineStateWithDescriptor:descriptor.0
                                                                                                            error:&mut err];

            match pipeline_state.is_null() {
                true => {
                    let desc: id = msg_send![err.0, localizedDescription];
                    let compile_error: *const libc::c_char = msg_send![desc.0, UTF8String];
                    Err(CStr::from_ptr(compile_error).to_string_lossy().into_owned())
                }
                false => Ok(pipeline_state)
            }
        }
    }

    pub fn new_buffer(&self, length: u64, options: MTLResourceOptions) -> MTLBuffer {
        unsafe {
            msg_send![self.0, newBufferWithLength:length
                                          options:options]
        }
    }

    pub fn new_buffer_with_data(&self, bytes: *const libc::c_void, length: NSUInteger, options: MTLResourceOptions) -> MTLBuffer {
        unsafe {
            msg_send![self.0, newBufferWithBytes:bytes
                                          length:length
                                         options:options]
        }
    }

    pub fn new_texture(&self, descriptor: MTLTextureDescriptor) -> MTLTexture {
        unsafe {
            msg_send![self.0, newTextureWithDescriptor:descriptor.0]
        }
    }

    pub fn new_sampler(&self, descriptor: MTLSamplerDescriptor) -> MTLSamplerState {
        unsafe {
            msg_send![self.0, newSamplerStateWithDescriptor:descriptor.0]
        }
    }

    pub fn new_heap(&self, descriptor: MTLHeapDescriptor) -> MTLHeap {
        unsafe {
            msg_send![self.0, newHeapWithDescriptor:descriptor.0]
        }
    }

    pub fn heap_buffer_size_and_align_with_length(&self, length: NSUInteger, options: MTLResourceOptions) -> MTLSizeAndAlign {
        unsafe {
            msg_send![self.0, heapBufferSizeAndAlignWithLength:length
                                                       options:options]
        }
    }

    pub fn heap_texture_size_and_align_with_descriptor(&self, desc: MTLTextureDescriptor) -> MTLSizeAndAlign {
        unsafe {
            msg_send![self.0, heapTextureSizeAndAlignWithDescriptor:desc.0]
        }
    }

    pub fn new_depth_stencil_state(&self, descriptor: MTLDepthStencilDescriptor) -> MTLDepthStencilState {
        unsafe {
            msg_send![self.0, newDepthStencilStateWithDescriptor:descriptor]
        }
    }

    pub fn new_argument_encoder_with_arguments(&self, arguments: NSArray<MTLArgumentDescriptor>) -> MTLArgumentEncoder {
        unsafe {
            msg_send![self.0, newArgumentEncoderWithArguments:arguments.0]
        }
    }
}

impl NSObjectProtocol for MTLDevice {
    unsafe fn class() -> &'static Class {
        Class::get("MTLDevice").unwrap()
    }
}

pub enum MTLFencePrototype {}
pub type MTLFence = id<(MTLFencePrototype, (NSObjectPrototype, ()))>;

impl<'a> MTLFence {
    pub fn set_label(&self, label: &str) {
        unsafe {
            let nslabel = NSString::from_str(label);
            msg_send![self.0, setLabel:transmute_copy::<_, *const ()>(&nslabel)]
        }
    }
}

impl NSObjectProtocol for MTLFence {
    unsafe fn class() -> &'static Class {
        Class::get("MTLFence").unwrap()
    }
}
