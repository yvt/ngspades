//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Device object.
use std::sync::Arc;

use crate::{arg, command, heap, limits, pass, pipeline, resources, sampler, shader, sync};
use crate::{ArgArrayIndex, ArgIndex, MemoryType};
use crate::{Object, Result};

/// A boxed handle representing a device object.
pub type DeviceRef = Arc<dyn Device>;

/// Trait for device objects.
///
/// The lifetime of the underlying device object is associated with that of
/// `Device`. Drop the `Device` to destroy the associated device object
/// (cf. handle types).
pub trait Device: Object {
    fn caps(&self) -> &dyn limits::DeviceCaps;

    /// Retrieve a reference to a global heap of the specified memory type,
    /// maintained by this device.
    ///
    /// A global heap is a special kind of heap that supports dynamic allocation
    /// (like dynamic heaps) and automatic resizing. When a resource bound to
    /// a global heap is released, the memory region allocated to it is
    /// automatically reclaimed (as if `make_aliases` is called).
    fn global_heap(&self, memory_type: MemoryType) -> &heap::HeapRef;

    /// Create a `CmdQueueBuilder` associated with this device.
    fn build_cmd_queue(&self) -> command::CmdQueueBuilderRef;

    /// Create a `SemaphoreBuilder` associated with this device.
    ///
    /// `DeviceExt` provides a shorthand method named [`new_semaphore`].
    ///
    /// The default implementation returns a [`NotSupportedSemaphoreBuilder`].
    ///
    /// [`new_semaphore`]: DeviceExt::new_semaphore
    /// [`NotSupportedSemaphoreBuilder`]: crate::sync::NotSupportedSemaphoreBuilder
    fn build_semaphore(&self) -> sync::SemaphoreBuilderRef {
        Box::new(sync::NotSupportedSemaphoreBuilder)
    }

    /// Create a `DynamicHeapBuilder` associated with this device.
    fn build_dynamic_heap(&self) -> heap::DynamicHeapBuilderRef;

    /// Create a `DedicatedHeapBuilder` associated with this device.
    fn build_dedicated_heap(&self) -> heap::DedicatedHeapBuilderRef;

    /// Create an `ImageBuilder` associated with this device.
    fn build_image(&self) -> resources::ImageBuilderRef;

    /// Create a `BufferBuilder` associated with this device.
    fn build_buffer(&self) -> resources::BufferBuilderRef;

    /// Create a `SamplerBuilder` associated with this device.
    fn build_sampler(&self) -> sampler::SamplerBuilderRef;

    /// Create a `LibraryBuilder` associated with this device.
    fn build_library(&self) -> shader::LibraryBuilderRef;

    /// Create a `ArgTableSigBuilder` associated with this device.
    fn build_arg_table_sig(&self) -> arg::ArgTableSigBuilderRef;

    /// Create a `RootSigBuilder` associated with this device.
    fn build_root_sig(&self) -> arg::RootSigBuilderRef;

    /// Create a `ArgPoolBuilder` associated with this device.
    fn build_arg_pool(&self) -> arg::ArgPoolBuilderRef;

    /// Create a `RenderPassBuilder` associated with this device.
    fn build_render_pass(&self) -> pass::RenderPassBuilderRef;

    /// Create a `RenderTargetTableBuilder` associated with this device.
    fn build_render_target_table(&self) -> pass::RenderTargetTableBuilderRef;

    /// Create a `RenderPipelineBuilder` associated with this device.
    fn build_render_pipeline(&self) -> pipeline::RenderPipelineBuilderRef;

    /// Create a `ComputePipelineBuilder` associated with this device.
    fn build_compute_pipeline(&self) -> pipeline::ComputePipelineBuilderRef;

    /// Update given argument tables.
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::*;
    ///     # fn test(
    ///     #     device: &Device,
    ///     #     arg_pool: &ArgPoolRef,
    ///     #     arg_table: &ArgTableRef,
    ///     #     arg_table_sig: &ArgTableSigRef,
    ///     #     images: &[&ImageRef],
    ///     #     buffer: &BufferRef
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
        arg_table_sig: &arg::ArgTableSigRef,
        updates: &[((&arg::ArgPoolRef, &arg::ArgTableRef), &[ArgUpdateSet<'_>])],
    ) -> Result<()>;

    /// Update a given argument table.
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::*;
    ///     # fn test(
    ///     #     device: &Device,
    ///     #     arg_pool: &ArgPoolRef,
    ///     #     arg_table: &ArgTableRef,
    ///     #     arg_table_sig: &ArgTableSigRef,
    ///     #     images: &[&ImageRef],
    ///     #     buffer: &BufferRef
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
        arg_table_sig: &arg::ArgTableSigRef,
        arg_pool: &arg::ArgPoolRef,
        arg_table: &arg::ArgTableRef,
        updates: &[ArgUpdateSet<'_>],
    ) -> Result<()> {
        self.update_arg_tables(arg_table_sig, &[((arg_pool, arg_table), updates)])
    }

    /// Create a autorelease pool and call the specified function inside it.
    ///
    /// On the macOS platform, the lifetimes of most Objective-C objects are
    /// managed by reference counting. In some cases, the lifetimes of objects
    /// are temporarily extended by inserting references to them into the
    /// current autorelease pool associated with each thread.
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
    fn autorelease_pool_scope_core(&self, cb: &mut dyn FnMut(&mut dyn AutoreleasePool)) {
        cb(&mut NullAutoreleasePool);
    }
}

/// Utilies for [`Device`](Device).
pub trait DeviceExt: Device {
    /// Create a `LibraryRef` associated with this device using a supplied SPIRV
    /// code.
    ///
    /// This is a shorthand method for [`build_library`].
    ///
    /// [`build_library`]: Device::build_library
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::*;
    ///     use zangfx_base::prelude::*;
    ///     # fn test(device: &Device) {
    ///     device
    ///         .new_library(&[])
    ///         .expect_err("Succeeded to create a shader library with an \
    ///                      invalid SPIR-V code.");
    ///     # }
    ///
    fn new_library(&self, spirv_code: &[u32]) -> Result<shader::LibraryRef> {
        self.build_library().spirv_code(spirv_code).build()
    }

    /// Create a `SemaphoreRef` associated with this device.
    ///
    /// This is a shorthand method for [`build_semaphore`].
    ///
    /// [`build_semaphore`]: Device::build_semaphore
    ///
    /// # Examples
    ///
    ///     # use zangfx_base::device::Device;
    ///     use zangfx_base::prelude::*;
    ///     # fn test(device: &Device) {
    ///     let semaphore = device.new_semaphore().unwrap();
    ///     # }
    ///
    fn new_semaphore(&self) -> Result<sync::SemaphoreRef> {
        self.build_semaphore().build()
    }

    /// Create a autorelease pool and call the specified function inside it.
    ///
    /// This is a wrapper of [`autorelease_pool_scope_core`] that allows the function
    /// to return a value. See the documentation of `autorelease_pool_scope_core` for
    /// details.
    ///
    /// [`autorelease_pool_scope_core`]: Device::autorelease_pool_scope_core
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
        T: FnOnce(&mut dyn AutoreleasePool) -> S,
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

impl<T: ?Sized + Device> DeviceExt for T {}

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
pub type ArgUpdateSet<'a> = (ArgIndex, ArgArrayIndex, resources::ArgSlice<'a>);

/// An autorelease pool.
///
/// See [`Device::autorelease_pool_scope_core`] for more.
pub trait AutoreleasePool {
    fn drain(&mut self);
}

/// The implementation of `AutoreleasePool` for platforms where the management of
/// autorelease pools are unnecessary.
pub struct NullAutoreleasePool;
impl AutoreleasePool for NullAutoreleasePool {
    fn drain(&mut self) {}
}
