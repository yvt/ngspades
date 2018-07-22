//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Device object.
use std::sync::Arc;

use crate::{arg, command, heap, limits, pass, pipeline, resources, sampler, shader, sync};
use crate::{ArgArrayIndex, ArgIndex};
use crate::{Object, Result};

/// A boxed handle representing a device object.
pub type Device = Arc<dyn DeviceTrait>;

/// Trait for device objects.
///
/// The lifetime of the underlying device object is associated with that of
/// `Device`. Drop the `Device` to destroy the associated device object
/// (cf. handle types).
pub trait DeviceTrait: Object {
    fn caps(&self) -> &limits::DeviceCaps;

    /// Retrieve a reference to the global heap maintained by this device.
    ///
    /// A global heap is a special kind of heap that supports dynamic allocation
    /// (like dynamic heaps) and automatic resizing. When a resource bound to
    /// a global heap is released, the memory region allocated to it is
    /// automatically reclaimed (as if `make_aliases` is called).
    fn global_heap(&self) -> &heap::Heap;

    /// Create a `CmdQueueBuilder` associated with this device.
    fn build_cmd_queue(&self) -> command::CmdQueueBuilder;

    /// Create a `SemaphoreBuilder` associated with this device.
    ///
    /// `DeviceExt` provides a shorthand method named [`new_semaphore`].
    ///
    /// The default implementation returns a [`NotSupportedSemaphoreBuilder`].
    ///
    /// [`new_semaphore`]: DeviceExt::new_semaphore
    /// [`NotSupportedSemaphoreBuilder`]: crate::sync::NotSupportedSemaphoreBuilder
    fn build_semaphore(&self) -> sync::SemaphoreBuilder {
        Box::new(sync::NotSupportedSemaphoreBuilder)
    }

    /// Create a `DynamicHeapBuilder` associated with this device.
    fn build_dynamic_heap(&self) -> heap::DynamicHeapBuilder;

    /// Create a `DedicatedHeapBuilder` associated with this device.
    fn build_dedicated_heap(&self) -> heap::DedicatedHeapBuilder;

    /// Create an `ImageBuilder` associated with this device.
    fn build_image(&self) -> resources::ImageBuilder;

    /// Create a `BufferBuilder` associated with this device.
    fn build_buffer(&self) -> resources::BufferBuilder;

    /// Create a `SamplerBuilder` associated with this device.
    fn build_sampler(&self) -> sampler::SamplerBuilder;

    /// Create a `LibraryBuilder` associated with this device.
    fn build_library(&self) -> shader::LibraryBuilder;

    /// Create a `ArgTableSigBuilder` associated with this device.
    fn build_arg_table_sig(&self) -> arg::ArgTableSigBuilder;

    /// Create a `RootSigBuilder` associated with this device.
    fn build_root_sig(&self) -> arg::RootSigBuilder;

    /// Create a `ArgPoolBuilder` associated with this device.
    fn build_arg_pool(&self) -> arg::ArgPoolBuilder;

    /// Create a `RenderPassBuilder` associated with this device.
    fn build_render_pass(&self) -> pass::RenderPassBuilder;

    /// Create a `RenderTargetTableBuilder` associated with this device.
    fn build_render_target_table(&self) -> pass::RenderTargetTableBuilder;

    /// Create a `RenderPipelineBuilder` associated with this device.
    fn build_render_pipeline(&self) -> pipeline::RenderPipelineBuilder;

    /// Create a `ComputePipelineBuilder` associated with this device.
    fn build_compute_pipeline(&self) -> pipeline::ComputePipelineBuilder;

    /// Update given argument tables.
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::*;
    ///     # fn test(
    ///     #     device: &Device,
    ///     #     arg_pool: &ArgPool,
    ///     #     arg_table: &ArgTable,
    ///     #     arg_table_sig: &ArgTableSig,
    ///     #     images: &[&Image],
    ///     #     buffer: &Buffer
    ///     # ) {
    ///     device.update_arg_tables(
    ///         arg_table_sig,
    ///         &[(
    ///             (arg_pool, arg_table),
    ///             &[
    ///                 // The index range 0..2 of the argument 0
    ///                 (0, 0, [images[0], images[1]][..].into()),
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
        arg_table_sig: &arg::ArgTableSig,
        updates: &[((&arg::ArgPool, &arg::ArgTable), &[ArgUpdateSet])],
    ) -> Result<()>;

    /// Update a given argument table.
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::{Device, Image, Buffer, ArgPool, ArgTable, ArgTableSig};
    ///     # fn test(
    ///     #     device: &Device,
    ///     #     arg_pool: &ArgPool,
    ///     #     arg_table: &ArgTable,
    ///     #     arg_table_sig: &ArgTableSig,
    ///     #     images: &[&Image],
    ///     #     buffer: &Buffer
    ///     # ) {
    ///     device.update_arg_table(
    ///         arg_table_sig,
    ///         arg_pool,
    ///         arg_table,
    ///         &[
    ///             // The index range 0..2 of the argument 0
    ///             (0, 0, [images[0], images[1]][..].into()),
    ///
    ///             // The index range 2..3 of the argument 1
    ///             (1, 2, [(0..1024, buffer)][..].into()),
    ///         ],
    ///     );
    ///     # }
    ///
    fn update_arg_table(
        &self,
        arg_table_sig: &arg::ArgTableSig,
        arg_pool: &arg::ArgPool,
        arg_table: &arg::ArgTable,
        updates: &[ArgUpdateSet],
    ) -> Result<()> {
        self.update_arg_tables(arg_table_sig, &[((arg_pool, arg_table), updates)])
    }

    /// Create a autorelease pool and call the specified function inside it.
    ///
    /// On the macOS platform, the lifetimes of most Cocoa objects are managed by
    /// reference counting. In some cases, the lifetimes of objects are temporarily
    /// extended by inserting references to them into the current autorelease pool
    /// associated with each thread.
    ///
    /// In standard macOS applications, a default autorelease pool is automatically
    /// provided and it is drained at every cycle of the event loop. However,
    /// this is unlikely to be the case in NgsGFX applications. Without an
    /// autorelease pool, autoreleased objects will never get released and you will
    /// leak memory.
    ///
    /// This function provides applications a method to create an
    /// autorelease pool in a cross-platform manner. You must wrap the main event
    /// loop with this function and drain the autorelease pool periodicaly
    /// (by calling `AutoreleasePool::drain`), for example, for every iteration.
    ///
    /// The default implementation just calls the given function with
    /// a mutable reference to [`NullAutoreleasePool`] as the parameter value.
    /// It is expected that the Metal backend is the only backend that provides
    /// a custom implementation of this function.
    ///
    /// [`NullAutoreleasePool`]: NullAutoreleasePool
    ///
    /// [`DeviceExt`] provides a helper function [`autorelease_pool_scope`] that
    /// allows to use this method with a callback function that returns a value.
    ///
    /// [`DeviceExt`]: DeviceExt
    /// [`autorelease_pool_scope`]: DeviceExt::autorelease_pool_scope
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::device::Device;
    ///     # fn test(device: &Device) {
    ///     device.autorelease_pool_scope_core(&mut |pool| {
    ///         loop {
    ///             // Perform tasks here
    ///             pool.drain();
    ///         }
    ///     });
    ///     # }
    ///
    fn autorelease_pool_scope_core(&self, cb: &mut FnMut(&mut AutoreleasePool)) {
        cb(&mut NullAutoreleasePool);
    }
}

/// Utilies for [`DeviceTrait`](DeviceTrait).
pub trait DeviceExt: DeviceTrait {
    /// Create a `Library` associated with this device using a supplied SPIRV
    /// code.
    ///
    /// This is a shorthand method for [`build_library`].
    ///
    /// [`build_library`]: DeviceTrait::build_library
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::*;
    ///     use zangfx_base::prelude::*;
    ///     # fn test(device: &DeviceTrait) {
    ///     device
    ///         .new_library(&[])
    ///         .expect_err("Succeeded to create a shader library with an \
    ///                      invalid SPIR-V code.");
    ///     # }
    ///
    fn new_library(&self, spirv_code: &[u32]) -> Result<shader::Library> {
        self.build_library().spirv_code(spirv_code).build()
    }

    /// Create a `Semaphore` associated with this device.
    ///
    /// This is a shorthand method for [`build_semaphore`].
    ///
    /// [`build_semaphore`]: DeviceTrait::build_semaphore
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::device::Device;
    ///     use zangfx_base::prelude::*;
    ///     # fn test(device: &Device) {
    ///     let semaphore = device.new_semaphore().unwrap();
    ///     # }
    ///
    fn new_semaphore(&self) -> Result<sync::Semaphore> {
        self.build_semaphore().build()
    }

    /// Create a autorelease pool and call the specified function inside it.
    ///
    /// This is a wrapper of [`autorelease_pool_scope_core`] that allows the function
    /// to return a value. See the documentation of `autorelease_pool_scope_core` for
    /// details.
    ///
    /// [`autorelease_pool_scope_core`]: DeviceTrait::autorelease_pool_scope_core
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::device::Device;
    ///     use zangfx_base::prelude::*;
    ///     # fn test(device: &Device) {
    ///     device.autorelease_pool_scope(|pool| {
    ///         // Perform tasks here
    ///         Some(())
    ///     }).unwrap();
    ///     # }
    ///
    fn autorelease_pool_scope<T, S>(&self, cb: T) -> S
    where
        T: FnOnce(&mut AutoreleasePool) -> S,
    {
        use std::mem::replace;
        enum State<T, S> {
            Before(T),
            Intermediate,
            After(S),
        }

        let mut state = State::Before(cb);

        self.autorelease_pool_scope_core(&mut |pool| {
            let func = match replace(&mut state, State::Intermediate) {
                State::Before(func) => func,
                State::Intermediate => unreachable!(),
                State::After(_) => panic!("callback function was called twice"),
            };
            state = State::After(func(pool));
        });

        match state {
            State::Before(_) => panic!("callback function was not called"),
            State::Intermediate => unreachable!(),
            State::After(value) => value,
        }
    }
}

impl<T: ?Sized + DeviceTrait> DeviceExt for T {}

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
/// See the documentation of [`update_arg_table`](DeviceTrait::update_arg_table) for
/// example.
pub type ArgUpdateSet<'a> = (ArgIndex, ArgArrayIndex, resources::ArgSlice<'a>);

/// An autorelease pool.
///
/// See [`Backend::autorelease_pool_scope`] for more.
///
/// [`Backend::autorelease_pool_scope`]: trait.Backend.html#method.autorelease_pool_scope
pub trait AutoreleasePool {
    fn drain(&mut self);
}

/// The implementation of `AutoreleasePool` for platforms where the management of
/// autorelease pools are unnecessary.
pub struct NullAutoreleasePool;
impl AutoreleasePool for NullAutoreleasePool {
    fn drain(&mut self) {}
}
