//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Device object.
use Object;
use common::Result;
use {arg, command, handles, heap, limits, pass, pipeline, resources, sampler, shader, sync};
use {ArgArrayIndex, ArgIndex};

/// Trait for device objects.
///
/// The lifetime of the underlying device object is associated with that of
/// `Device`. Drop the `Device` to destroy the associated device object
/// (cf. handle types).
pub trait Device: Object {
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

    /// Update given argument tables.
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::device::Device;
    ///     # use zangfx_base::handles::{ImageView, Buffer, ArgTable, ArgTableSig};
    ///     # fn test(
    ///     #     device: &Device,
    ///     #     arg_table: &ArgTable,
    ///     #     arg_table_sig: &ArgTableSig,
    ///     #     image_views: &[&ImageView],
    ///     #     buffer: &Buffer
    ///     # ) {
    ///     device.update_arg_tables(
    ///         arg_table_sig,
    ///         &[(
    ///             arg_table,
    ///             &[
    ///                 // The index range 0..2 of the argument 0
    ///                 (0, 0, [image_views[0], image_views[1]][..].into()),
    ///
    ///                 // The index range 2..3 of the argument 1
    ///                 (1, 2, [(0..1024, buffer)][..].into()),
    ///             ],
    ///         )],
    ///     );
    ///     # }
    ///
    fn update_arg_tables(
        &self,
        arg_table_sig: &handles::ArgTableSig,
        updates: &[(&handles::ArgTable, &[ArgUpdateSet])],
    ) -> Result<()>;

    /// Update a given argument table.
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::device::Device;
    ///     # use zangfx_base::handles::{ImageView, Buffer, ArgTable, ArgTableSig};
    ///     # fn test(
    ///     #     device: &Device,
    ///     #     arg_table: &ArgTable,
    ///     #     arg_table_sig: &ArgTableSig,
    ///     #     image_views: &[&ImageView],
    ///     #     buffer: &Buffer
    ///     # ) {
    ///     device.update_arg_table(
    ///         arg_table_sig,
    ///         arg_table,
    ///         &[
    ///             // The index range 0..2 of the argument 0
    ///             (0, 0, [image_views[0], image_views[1]][..].into()),
    ///
    ///             // The index range 2..3 of the argument 1
    ///             (1, 2, [(0..1024, buffer)][..].into()),
    ///         ],
    ///     );
    ///     # }
    ///
    fn update_arg_table(
        &self,
        arg_table_sig: &handles::ArgTableSig,
        arg_table: &handles::ArgTable,
        updates: &[ArgUpdateSet],
    ) -> Result<()> {
        self.update_arg_tables(arg_table_sig, &[(arg_table, updates)])
    }
}

/// Utilies for [`Device`](Device).
pub trait DeviceExt: Device {
    // No methods are currently defined.
}

/// Represents a consecutive update of arguments in an argument table.
///
/// An `ArgUpdateSet` is comprised of the following parts:
///
///  - An `ArgIndex` specifying the argument index.
///  - An `ArgArrayIndex` specifying the starting index.
///  - An `ArgSlice` specifying the new contents.
///
/// Unlike Vulkan's descriptor update, `ArgSlice` does not overflow into the
/// succeeding argument slots. (This is prohibited in ZanGFX.)
///
/// See the documentation of [`update_arg_table`](Device::update_arg_table) for
/// example.
pub type ArgUpdateSet<'a> = (ArgIndex, ArgArrayIndex, handles::ArgSlice<'a>);

impl<T: ?Sized + Device> DeviceExt for T {}
