//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Device object.
use std::any::Any;
use std::fmt::Debug;
use common::Result;
use {arg, command, handles, heap, limits, pass, pipeline, resources, sampler, shader, sync};

/// Trait for device objects.
///
/// The lifetime of the underlying device object is associated with that of
/// `Device`. Drop the `Device` to destroy the associated device object
/// (cf. handle types).
pub trait Device: Send + Sync + Any + Debug + AsRef<Any> + AsMut<Any> {
    fn caps(&self) -> &limits::DeviceCaps;

    /// Create a `CmdQueueBuilder` associated with this device.
    fn build_cmd_queue(&self) -> Box<command::CmdQueueBuilder>;

    /// Create a `HeapBuilder` associated with this device.
    fn build_heap(&self) -> Box<heap::HeapBuilder>;

    /// Create an `BarrierBuilder` associated with this device.
    fn build_barrier(&self) -> Box<sync::BarrierBuilder>;

    /// Create an `ImageBuilder` associated with this device.
    fn build_image(&self) -> Box<resources::ImageBuilder>;

    /// Create a `BufferBuilder` associated with this device.
    fn build_buffer(&self) -> Box<resources::BufferBuilder>;

    /// Create a `SamplerBuilder` associated with this device.
    fn build_sampler(&self) -> Box<sampler::SamplerBuilder>;

    /// Create a `LibraryBuilder` associated with this device.
    fn build_library(&self) -> Box<shader::LibraryBuilder>;

    /// Create a `ArgTableSigBuilder` associated with this device.
    fn build_arg_table_sig(&self) -> Box<arg::ArgTableSigBuilder>;

    /// Create a `RootSigBuilder` associated with this device.
    fn build_root_sig(&self) -> Box<arg::RootSigBuilder>;

    /// Create a `ArgPoolBuilder` associated with this device.
    fn build_arg_pool(&self) -> Box<arg::ArgPoolBuilder>;

    /// Create a `RenderPassBuilder` associated with this device.
    fn build_render_pass(&self) -> Box<pass::RenderPassBuilder>;

    /// Create a `RtTableBuilder` associated with this device.
    fn build_rt_table(&self) -> Box<pass::RtTableBuilder>;

    // TODO: image view

    // TODO: render pipeline

    /// Create a `ComputePipelineBuilder` associated with this device.
    fn build_compute_pipeline(&self) -> Box<pipeline::ComputePipelineBuilder>;

    /// Destroy an `Image` associated with this device.
    fn destroy_image(&self, obj: &handles::Image) -> Result<()>;

    /// Destroy a `Buffer` associated with this device.
    fn destroy_buffer(&self, obj: &handles::Buffer) -> Result<()>;

    /// Destroy a `Sampler` associated with this device.
    fn destroy_sampler(&self, obj: &handles::Sampler) -> Result<()>;

    /// Retrieve the memory requirements for a given resource.
    fn get_memory_req(&self, obj: handles::ResourceRef) -> Result<resources::MemoryReq>;
}

/// Utilies for [`Device`](Device).
pub trait DeviceExt: Device {
    // No methods are currently defined.
}

impl<T: ?Sized + Device> DeviceExt for T {}
