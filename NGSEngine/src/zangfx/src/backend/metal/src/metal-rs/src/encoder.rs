use cocoa::foundation::{NSUInteger, NSRange};
use objc::runtime::Class;
use objc_foundation::{NSString, INSString};

use super::{id, NSObjectPrototype, NSObjectProtocol};

use libc;
use std::mem::transmute_copy;

use resource::{MTLResource, MTLHeap};
use texture::MTLTexture;
use buffer::MTLBuffer;
use pipeline::{MTLRenderPipelineState, MTLComputePipelineState};
use sampler::MTLSamplerState;
use depthstencil::MTLDepthStencilState;
use types::{MTLSize, MTLOrigin};
use device::MTLFence;

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MTLPrimitiveType {
    Point = 0,
    Line = 1,
    LineStrip = 2,
    Triangle = 3,
    TriangleStrip = 4,
}

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MTLIndexType {
   UInt16 = 0,
   UInt32 = 1,
}

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MTLVisibilityResultMode {
    Disabled = 0,
    Boolean = 1,
    Counting = 2,
}

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MTLCullMode {
    None = 0,
    Front = 1,
    Back = 2,
}

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MTLWinding {
    Clockwise = 0,
    CounterClockwise = 1,
}

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MTLDepthClipMode {
    Clip = 0,
    Clamp = 1,
}

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MTLTriangleFillMode {
    Fill = 0,
    Lines = 1,
}

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MTLResourceUsage {
    Read = 1 << 0,
    Write = 1 << 1,
    Sample = 1 << 2,
}

bitflags! {
    pub flags MTLBlitOption: NSUInteger {
        const MTLBlitOptionNone                    = 0,
        const MTLBlitOptionDepthFromDepthStencil   = 1 << 0,
        const MTLBlitOptionStencilFromDepthStencil = 1 << 1,
        const MTLBlitOptionRowLinearPVRTC          = 1 << 2
    }
}

bitflags! {
    pub flags MTLRenderStages: NSUInteger {
        const MTLRenderStageVertex   = 1 << 0,
        const MTLRenderStageFragment = 1 << 1,
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MTLScissorRect {
    pub x: NSUInteger,
    pub y: NSUInteger,
    pub width: NSUInteger,
    pub height: NSUInteger
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MTLViewport {
    pub originX: f64,
    pub originY: f64,
    pub width: f64,
    pub height: f64,
    pub znear: f64,
    pub zfar: f64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MTLDrawPrimitivesIndirectArguments {
    pub vertexCount: u32,
    pub instanceCount: u32,
    pub vertexStart: u32,
    pub baseInstance: u32
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MTLDrawIndexedPrimitivesIndirectArguments {
    pub indexCount: u32,
    pub instanceCount: u32,
    pub indexStart: u32,
    pub baseVertex: i32,
    pub baseInstance: u32
}

pub enum MTLCommandEncoderPrototype {}
pub type MTLCommandEncoder = id<
    (MTLCommandEncoderPrototype,
        (NSObjectPrototype, ()))>;

impl<'a> MTLCommandEncoder {
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

    pub fn insert_debug_signpost(&self, string: &str) {
        unsafe {
            let nsstring = NSString::from_str(string);
            msg_send![self.0, insertDebugSignpost:transmute_copy::<_, *const ()>(&nsstring)]
        }
    }

    pub fn push_debug_group(&self, string: &str) {
        unsafe {
            let nsstring = NSString::from_str(string);
            msg_send![self.0, pushDebugGroup:transmute_copy::<_, *const ()>(&nsstring)]
        }
    }

    pub fn pop_debug_group(&self) {
        unsafe {
            msg_send![self.0, popDebugGroup]
        }
    }

    pub fn end_encoding(&self) {
        unsafe {
            msg_send![self.0, endEncoding]
        }
    }
}

impl NSObjectProtocol for MTLCommandEncoder {
    unsafe fn class() -> &'static Class {
        Class::get("MTLCommandEncoder").unwrap()
    }
}

pub enum MTLParallelRenderCommandEncoderPrototype {}
pub type MTLParallelRenderCommandEncoder = id<
    (MTLParallelRenderCommandEncoderPrototype,
        (MTLCommandEncoderPrototype,
            (NSObjectPrototype, ())))>;

impl MTLParallelRenderCommandEncoder {
    pub fn render_command_encoder(&self) -> MTLRenderCommandEncoder {
        unsafe {
            msg_send![self.0, renderCommandEncoder]
        }
    }
}

impl NSObjectProtocol for MTLParallelRenderCommandEncoder {
    unsafe fn class() -> &'static Class {
        Class::get("MTLParallelRenderCommandEncoder").unwrap()
    }
}

pub enum MTLRenderCommandEncoderPrototype {}
pub type MTLRenderCommandEncoder = id<
    (MTLRenderCommandEncoderPrototype,
        (MTLCommandEncoderPrototype,
            (NSObjectPrototype, ())))>;

impl MTLRenderCommandEncoder {
    // Setting Graphics Rendering State

    pub fn set_render_pipeline_state(&self, pipeline_state: MTLRenderPipelineState) {
        unsafe {
            msg_send![self.0, setRenderPipelineState:pipeline_state.0]
        }
    }

    pub fn set_viewport(&self, viewport: MTLViewport) {
        unsafe {
            msg_send![self.0, setViewport:viewport]
        }
    }

    pub fn set_viewports(&self, viewports: &[MTLViewport]) {
        unsafe {
            msg_send![self.0, setViewports:viewports.as_ptr()
                                     count:viewports.len() as NSUInteger]
        }
    }

    pub fn set_front_facing_winding(&self, winding: MTLWinding) {
        unsafe {
            msg_send![self.0, setFrontFacingWinding:winding]
        }
    }

    pub fn set_cull_mode(&self, mode: MTLCullMode) {
        unsafe {
            msg_send![self.0, setCullMode:mode]
        }
    }

    pub fn set_depth_clip_mode(&self, mode: MTLDepthClipMode) {
        unsafe {
            msg_send![self.0, setDepthClipMode:mode]
        }
    }

    pub fn set_depth_bias(&self, bias: f32, scale: f32, clamp: f32) {
        unsafe {
            msg_send![self.0, setDepthBias:bias
                              slopeScale:scale
                                   clamp:clamp]
        }
    }

    pub fn set_scissor_rect(&self, rect: MTLScissorRect) {
        unsafe {
            msg_send![self.0, setScissorRect:rect]
        }
    }

    pub fn set_triangle_fill_mode(&self, mode: MTLTriangleFillMode) {
        unsafe {
            msg_send![self.0, setTriangleFillMode:mode]
        }
    }

    pub fn set_blend_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        unsafe {
            msg_send![self.0, setBlendColorRed:red
                                         green:green
                                          blue:blue
                                         alpha:alpha]
        }
    }

    pub fn set_depth_stencil_state(&self, depth_stencil_state: MTLDepthStencilState) {
        unsafe {
            msg_send![self.0, setDepthStencilState:depth_stencil_state.0]
        }
    }

    pub fn set_stencil_reference_value(&self, value: u32) {
        unsafe {
            msg_send![self.0, setStencilReferenceValue:value]
        }
    }

    pub fn set_stencil_front_back_reference_value(&self, front: u32, back: u32) {
        unsafe {
            msg_send![self.0, setStencilFrontReferenceValue:front
                                         backReferenceValue:back]
        }
    }

    pub fn set_visibility_result_mode(&self, offset: u64, mode: MTLVisibilityResultMode) {
        unsafe {
            msg_send![self.0, setVisibilityResultMode:mode
                                               offset:offset]
        }
    }

    // Specifying Resources for a Vertex Shader Function

    pub fn set_vertex_bytes(&self, index: u64, length: u64, bytes: *const libc::c_void) {
        unsafe {
            msg_send![self.0, setVertexBytes:bytes
                                      length:length
                                     atIndex:index]
        }
    }

    pub fn set_vertex_buffer(&self, index: u64, offset: u64, buffer: MTLBuffer) {
        unsafe {
            msg_send![self.0, setVertexBuffer:buffer.0
                                       offset:offset
                                      atIndex:index]
        }
    }

    pub fn set_vertex_buffer_offset(&self, index: u64, offset: u64) {
        unsafe {
            msg_send![self.0, setVertexBufferOffset:offset
                                            atIndex:index]
        }
    }

    pub fn set_vertex_buffers(&self, index: u64, buffers: &[MTLBuffer], offsets: &[u64]) {
        debug_assert_eq!(buffers.len(), offsets.len());
        unsafe {
            msg_send![self.0, setVertexBuffers:buffers.as_ptr()
                                       offsets:offsets.as_ptr()
                                     withRange:NSRange::new(index, buffers.len() as u64)]
        }
    }

    pub fn set_vertex_texture(&self, index: u64, texture: MTLTexture) {
        unsafe {
            msg_send![self.0, setVertexTexture:texture.0
                                       atIndex:index]
        }
    }

    pub fn set_vertex_sampler_state(&self, index: u64, sampler: MTLSamplerState) {
        unsafe {
            msg_send![self.0, setVertexSamplerState:sampler.0
                                            atIndex:index]
        }
    }

    pub fn set_vertex_sampler_state_with_lod(&self, index: u64, lod_min_clamp: f32, lod_max_clamp: f32, sampler: MTLSamplerState) {
        unsafe {
            msg_send![self.0, setVertexSamplerState:sampler.0
                                        lodMinClamp:lod_min_clamp
                                        lodMaxClamp:lod_max_clamp
                                            atIndex:index]
        }
    }

    // Specifying Resources for a Fragment Shader Function

    pub fn set_fragment_bytes(&self, index: u64, length: u64, bytes: *const libc::c_void) {
        unsafe {
            msg_send![self.0, setFragmentBytes:bytes
                                        length:length
                                       atIndex:index]
        }
    }

    pub fn set_fragment_buffer(&self, index: u64, offset: u64, buffer: MTLBuffer) {
        unsafe {
            msg_send![self.0, setFragmentBuffer:buffer.0
                                         offset:offset
                                        atIndex:index]
        }
    }

    pub fn set_fragment_buffers(&self, index: u64, buffers: &[MTLBuffer], offsets: &[u64]) {
        debug_assert_eq!(buffers.len(), offsets.len());
        unsafe {
            msg_send![self.0, setFragmentBuffers:buffers.as_ptr()
                                         offsets:offsets.as_ptr()
                                       withRange:NSRange::new(index, buffers.len() as u64)]
        }
    }

    pub fn set_fragment_buffer_offset(&self, index: u64, offset: u64) {
        unsafe {
            msg_send![self.0, setFragmentBufferOffset:offset
                                              atIndex:index]
        }
    }

    pub fn set_fragment_texture(&self, index: u64, texture: MTLTexture) {
        unsafe {
            msg_send![self.0, setFragmentTexture:texture.0
                                         atIndex:index]
        }
    }

    pub fn set_fragment_sampler_state(&self, index: u64, sampler: MTLSamplerState) {
        unsafe {
            msg_send![self.0, setFragmentSamplerState:sampler.0
                                              atIndex:index]
        }
    }

    pub fn set_fragment_sampler_state_with_lod(&self, index: u64, lod_min_clamp: f32, lod_max_clamp: f32, sampler: MTLSamplerState) {
        unsafe {
            msg_send![self.0, setFragmentSamplerState:sampler.0
                                          lodMinClamp:lod_min_clamp
                                          lodMaxClamp:lod_max_clamp
                                              atIndex:index]
        }
    }

    // Drawing Geometric Primitives

    pub fn draw_primitives(&self, primitive_type: MTLPrimitiveType, vertex_start: u64, vertex_count: u64) {
        unsafe {
            msg_send![self.0, drawPrimitives:primitive_type
                                 vertexStart:vertex_start
                                 vertexCount:vertex_count]
        }
    }

    pub fn draw_primitives_instanced(&self, primitive_type: MTLPrimitiveType, vertex_start: u64, vertex_count: u64, instance_count: u64, base_instance: u64) {
        unsafe {
            msg_send![self.0, drawPrimitives:primitive_type
                                 vertexStart:vertex_start
                                 vertexCount:vertex_count
                               instanceCount:instance_count
                                baseInstance:base_instance]
        }
    }

    pub fn draw_indexed_primitives(&self, primitive_type: MTLPrimitiveType, index_count: u64, index_type: MTLIndexType, index_buffer: MTLBuffer, index_buffer_offset: u64) {
        unsafe {
            msg_send![self.0, drawIndexedPrimitives:primitive_type
                                         indexCount:index_count
                                          indexType:index_type
                                        indexBuffer:index_buffer.0
                                  indexBufferOffset:index_buffer_offset]
        }
    }

    pub fn draw_indexed_primitives_instanced(&self, primitive_type: MTLPrimitiveType, index_count: u64, index_type: MTLIndexType, index_buffer: MTLBuffer, index_buffer_offset: u64, instance_count: u64, base_vertex: i64, base_instance: u64) {
        unsafe {
            msg_send![self.0, drawIndexedPrimitives:primitive_type
                                         indexCount:index_count
                                          indexType:index_type
                                        indexBuffer:index_buffer.0
                                  indexBufferOffset:index_buffer_offset
                                      instanceCount:instance_count
                                         baseVertex:base_vertex
                                       baseInstance:base_instance]
        }
    }

    pub fn draw_indirect(&self, primitive_type: MTLPrimitiveType, indirect_buffer: MTLBuffer, indirect_buffer_offset: u64) {
        unsafe {
            msg_send![self.0, drawPrimitives:primitive_type
                              indirectBuffer:indirect_buffer
                        indirectBufferOffset:indirect_buffer_offset]
        }
    }

    pub fn draw_indexed_indirect(&self, primitive_type: MTLPrimitiveType, index_type: MTLIndexType, index_buffer: MTLBuffer, index_buffer_offset: u64, indirect_buffer: MTLBuffer, indirect_buffer_offset: u64) {
        unsafe {
            msg_send![self.0, drawIndexedPrimitives:primitive_type
                                          indexType:index_type
                                        indexBuffer:index_buffer
                                  indexBufferOffset:index_buffer_offset
                                     indirectBuffer:indirect_buffer
                               indirectBufferOffset:indirect_buffer_offset]
        }
    }

    // fn setVertexBufferOffset_atIndex(self, offset: NSUInteger, index: NSUInteger);
    // fn setVertexBuffers_offsets_withRange(self, buffers: *const id, offsets: *const NSUInteger, range: NSRange);
    // fn setVertexTextures_withRange(self, textures: *const id, range: NSRange);
    // fn setVertexSamplerStates_withRange(self, samplers: *const id, range: NSRange);
    // fn setVertexSamplerStates_lodMinClamps_lodMaxClamps_withRange(self, samplers: *const id, lodMinClamps: *const f32, lodMaxClamps: *const f32, range: NSRange);

    // Performing Fence Operations

    pub fn update_fence_after_stages(&self, fence: MTLFence, stages: MTLRenderStages) {
        unsafe {
            msg_send![self.0, updateFence:fence.0
                              afterStages:stages]
        }
    }

    pub fn wait_for_fence_before_stages(&self, fence: MTLFence, stages: MTLRenderStages) {
        unsafe {
            msg_send![self.0, waitForFence:fence.0
                              beforeStages:stages]
        }
    }

    // Enabling Texture Barriers
    pub fn texture_barrier(&self) {
        unsafe {
            msg_send![self.0, textureBarrier]
        }
    }

    // Specifying Resources for an Argument Buffer

    pub fn use_resources(&self, resources: &[MTLResource], usage: MTLResourceUsage) {
        unsafe {
            msg_send![self.0, useResources:resources.as_ptr()
                                     count:resources.len() as NSUInteger
                                     usage:usage]
        }
    }

    pub fn use_heaps(&self, heaps: &[MTLHeap]) {
        unsafe {
            msg_send![self.0, useHeaps:heaps.as_ptr()
                                 count:heaps.len() as NSUInteger]
        }
    }
}

impl NSObjectProtocol for MTLRenderCommandEncoder {
    unsafe fn class() -> &'static Class {
        Class::get("MTLRenderCommandEncoder").unwrap()
    }
}

pub enum MTLBlitCommandEncoderPrototype {}
pub type MTLBlitCommandEncoder = id<
    (MTLBlitCommandEncoderPrototype,
        (MTLCommandEncoderPrototype,
            (NSObjectPrototype, ())))>;

impl MTLBlitCommandEncoder {

    pub fn update_fence(&self, fence: MTLFence) {
        unsafe {
            msg_send![self.0, updateFence:fence.0]
        }
    }

    pub fn wait_for_fence(&self, fence: MTLFence) {
        unsafe {
            msg_send![self.0, waitForFence:fence.0]
        }
    }

    pub fn synchronize_resource(&self, resource: MTLResource) {
        unsafe {
            msg_send![self.0, synchronizeResource:resource]
        }
    }

    pub fn fill_buffer(&self, buffer: MTLBuffer, range: NSRange, value: u8) {
        unsafe {
            msg_send![self.0, fillBuffer:buffer
                                   range:range
                                   value:value]
        }
    }

    pub fn copy_from_buffer_to_buffer(&self, source_buffer: MTLBuffer, source_offset: u64, destination_buffer: MTLBuffer, destination_offset: u64, size: u64) {
        unsafe {
            msg_send![self.0, copyFromBuffer:source_buffer
                                sourceOffset:source_offset
                                    toBuffer:destination_buffer
                           destinationOffset:destination_offset
                                        size:size]
        }
    }

    pub fn copy_from_buffer_to_image(
        &self,
        source_buffer: MTLBuffer,
        source_offset: u64,
        source_bytes_per_row: u64,
        source_bytes_per_image: u64,
        source_size: MTLSize,
        destination_texture: MTLTexture,
        destination_slice: u64,
        destination_level: u64,
        destination_origin: MTLOrigin,
        options: MTLBlitOption,
    ) {
        unsafe {
            msg_send![self.0, copyFromBuffer:source_buffer
                                sourceOffset:source_offset
                           sourceBytesPerRow:source_bytes_per_row
                         sourceBytesPerImage:source_bytes_per_image
                                  sourceSize:source_size
                                   toTexture:destination_texture
                            destinationSlice:destination_slice
                            destinationLevel:destination_level
                           destinationOrigin:destination_origin
                                     options:options]
        }
    }

    pub fn copy_from_image_to_buffer(
        &self,
        source_texture: MTLTexture,
        source_slice: u64,
        source_level: u64,
        source_origin: MTLOrigin,
        source_size: MTLSize,
        destination_buffer: MTLBuffer,
        destination_offset: u64,
        destination_bytes_per_row: u64,
        destination_bytes_per_image: u64,
        options: MTLBlitOption,
    ) {
        unsafe {
            msg_send![self.0, copyFromTexture:source_texture
                                  sourceSlice:source_slice
                                  sourceLevel:source_level
                                 sourceOrigin:source_origin
                                   sourceSize:source_size
                                     toBuffer:destination_buffer
                            destinationOffset:destination_offset
                       destinationBytesPerRow:destination_bytes_per_row
                     destinationBytesPerImage:destination_bytes_per_image
                                      options:options]
        }
    }

    pub fn copy_from_image_to_image(
        &self,
        source_texture: MTLTexture,
        source_slice: u64,
        source_level: u64,
        source_origin: MTLOrigin,
        source_size: MTLSize,
        destination_texture: MTLTexture,
        destination_slice: u64,
        destination_level: u64,
        destination_origin: MTLOrigin,
    ) {
        unsafe {
            msg_send![self.0, copyFromTexture:source_texture
                                  sourceSlice:source_slice
                                  sourceLevel:source_level
                                 sourceOrigin:source_origin
                                   sourceSize:source_size
                                    toTexture:destination_texture
                             destinationSlice:destination_slice
                             destinationLevel:destination_level
                            destinationOrigin:destination_origin]
        }
    }

}


impl NSObjectProtocol for MTLBlitCommandEncoder {
    unsafe fn class() -> &'static Class {
        Class::get("MTLBlitCommandEncoder").unwrap()
    }
}

pub enum MTLComputeCommandEncoderPrototype {}
pub type MTLComputeCommandEncoder = id<
    (MTLComputeCommandEncoderPrototype,
        (MTLCommandEncoderPrototype,
            (NSObjectPrototype, ())))>;

impl MTLComputeCommandEncoder {

    pub fn update_fence(&self, fence: MTLFence) {
        unsafe {
            msg_send![self.0, updateFence:fence.0]
        }
    }

    pub fn wait_for_fence(&self, fence: MTLFence) {
        unsafe {
            msg_send![self.0, waitForFence:fence.0]
        }
    }

    pub fn set_compute_pipeline_state(&self, pipeline_state: MTLComputePipelineState) {
        unsafe {
            msg_send![self.0, setComputePipelineState:pipeline_state.0]
        }
    }

    pub fn set_bytes(&self, index: u64, length: u64, bytes: *const libc::c_void) {
        unsafe {
            msg_send![self.0, setBytes:bytes
                                length:length
                                     atIndex:index]
        }
    }

    pub fn set_buffer(&self, index: u64, offset: u64, buffer: MTLBuffer) {
        unsafe {
            msg_send![self.0, setBuffer:buffer.0
                                 offset:offset
                                      atIndex:index]
        }
    }

    pub fn set_buffer_offset(&self, index: u64, offset: u64) {
        unsafe {
            msg_send![self.0, setBufferOffset:offset
                                      atIndex:index]
        }
    }

    pub fn set_buffers(&self, index: u64, buffers: &[MTLBuffer], offsets: &[u64]) {
        debug_assert_eq!(buffers.len(), offsets.len());
        unsafe {
            msg_send![self.0, setBuffers:buffers.as_ptr()
                                 offsets:offsets.as_ptr()
                               withRange:NSRange::new(index, buffers.len() as u64)]
        }
    }

    pub fn set_texture(&self, index: u64, texture: MTLTexture) {
        unsafe {
            msg_send![self.0, setTexture:texture.0
                                 atIndex:index]
        }
    }

    pub fn set_sampler_state(&self, index: u64, sampler: MTLSamplerState) {
        unsafe {
            msg_send![self.0, setSamplerState:sampler.0
                                      atIndex:index]
        }
    }

    pub fn set_sampler_state_with_lod(&self, index: u64, lod_min_clamp: f32, lod_max_clamp: f32, sampler: MTLSamplerState) {
        unsafe {
            msg_send![self.0, setSamplerState:sampler.0
                                  lodMinClamp:lod_min_clamp
                                  lodMaxClamp:lod_max_clamp
                                      atIndex:index]
        }
    }

    pub fn set_threadgroup_memory_length(&self, index: u64, length: u64) {
        unsafe {
            msg_send![self.0, setThreadgroupMemoryLength:length
                                                 atIndex:index]
        }
    }

    pub fn dispatch_threadgroups(&self, threadgroups_per_grid: MTLSize, threads_per_threadgroup: MTLSize) {
        unsafe {
            msg_send![self.0, dispatchThreadgroups:threadgroups_per_grid
                             threadsPerThreadgroup:threads_per_threadgroup]
        }
    }

    pub fn dispatch_threadgroups_with_indirect_buffer(&self, indirect_buffer: MTLBuffer, indirect_buffer_offset: u64, threads_per_threadgroup: MTLSize) {
        unsafe {
            msg_send![self.0, dispatchThreadgroupsWithIndirectBuffer:indirect_buffer.0
                                                indirectBufferOffset:indirect_buffer_offset
                                               threadsPerThreadgroup:threads_per_threadgroup]
        }
    }

    // Specifying Resources for an Argument Buffer

    pub fn use_resources(&self, resources: &[MTLResource], usage: MTLResourceUsage) {
        unsafe {
            msg_send![self.0, useResources:resources.as_ptr()
                                     count:resources.len() as NSUInteger
                                     usage:usage]
        }
    }

    pub fn use_heaps(&self, heaps: &[MTLHeap]) {
        unsafe {
            msg_send![self.0, useHeaps:heaps.as_ptr()
                                 count:heaps.len() as NSUInteger]
        }
    }
}


impl NSObjectProtocol for MTLComputeCommandEncoder {
    unsafe fn class() -> &'static Class {
        Class::get("MTLComputeCommandEncoder").unwrap()
    }
}

