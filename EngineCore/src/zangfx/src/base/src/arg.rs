//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for argument table objects, argument table signature objects, and
//! root signature objects, and other relevant types.
use std::sync::Arc;

use crate::command::CmdQueueRef;
use crate::resources::ImageAspect;
use crate::shader::ShaderStageFlags;
use crate::{ArgArrayIndex, ArgIndex, ArgTableIndex};
use crate::{Object, Result};

define_handle! {
    /// Argument set signature handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    ArgTableSigRef
}

define_handle! {
    /// Argument set handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    ArgTableRef
}

define_handle! {
    /// Root signature handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    RootSigRef
}

/// A builder object for argument table signature objects.
pub type ArgTableSigBuilderRef = Box<dyn ArgTableSigBuilder>;

/// Trait for building argument table signature objects.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device) {
///     let mut builder = device.build_arg_table_sig();
///     builder.arg(0, ArgType::SampledImage)
///         .set_stages(ShaderStageFlags::Fragment)
///         .set_len(16);
///     builder.arg(1, ArgType::Sampler)
///         .set_stages(ShaderStageFlags::Fragment);
///     builder.arg(2, ArgType::StorageBuffer);
///
///     let arg_table_sig = builder.build()
///         .expect("Failed to create an argument table signature.");
///     # }
///
pub trait ArgTableSigBuilder: Object {
    /// Define an argument. Use the returned `dyn ArgSig` to specify
    /// additional properties of it.
    fn arg(&mut self, index: ArgIndex, ty: ArgType) -> &mut dyn ArgSig;

    /// Build an `ArgTableSigRef`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<ArgTableSigRef>;
}

/// Trait for setting properties of an argument in an argument table signature.
pub trait ArgSig: Object {
    /// Set the number of elements. Must be non-zero.
    ///
    /// Defaults to `1`.
    fn set_len(&mut self, x: ArgArrayIndex) -> &mut dyn ArgSig;

    /// Set the set of shader stages from which this argument is used.
    ///
    /// Defaults to all shader stages supported by the backend.
    fn set_stages(&mut self, x: ShaderStageFlags) -> &mut dyn ArgSig;

    /// Set the image aspect.
    ///
    /// Defaults to `Color`. Must be `Color` or `Depth`.
    fn set_image_aspect(&mut self, _: ImageAspect) -> &mut dyn ArgSig;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArgType {
    StorageImage,
    SampledImage,
    Sampler,
    UniformBuffer,
    StorageBuffer,
}

impl ArgType {
    pub fn has_image_view(&self) -> bool {
        match *self {
            ArgType::StorageImage => true,
            ArgType::SampledImage => true,
            ArgType::Sampler => false,
            ArgType::UniformBuffer => false,
            ArgType::StorageBuffer => false,
        }
    }

    pub fn has_sampler(&self) -> bool {
        match *self {
            ArgType::StorageImage => false,
            ArgType::SampledImage => false,
            ArgType::Sampler => true,
            ArgType::UniformBuffer => false,
            ArgType::StorageBuffer => false,
        }
    }

    pub fn has_buffer(&self) -> bool {
        match *self {
            ArgType::StorageImage => false,
            ArgType::SampledImage => false,
            ArgType::Sampler => false,
            ArgType::UniformBuffer => true,
            ArgType::StorageBuffer => true,
        }
    }
}

/// A builder object for root signature objects.
pub type RootSigBuilderRef = Box<dyn RootSigBuilder>;

/// Trait for building root signature objects.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, arg_table_sig: &ArgTableSigRef) {
///     let root_sig = device.build_root_sig()
///         .arg_table(0, arg_table_sig)
///         .build()
///         .expect("Failed to create a root signature.");
///     # }
///
pub trait RootSigBuilder: Object {
    /// Set the argument table signature at the specified location.
    fn arg_table(&mut self, index: ArgTableIndex, x: &ArgTableSigRef) -> &mut dyn RootSigBuilder;

    /// Build an `RootSigRef`.
    ///
    /// # Valid Usage
    ///
    /// - All mandatory properties must have their values set before this method
    ///   is called.
    /// - Binding indices of argument table signatures must be tightly arranged.
    ///   That is, when `N` is max(binding indices ∪ -1), there must not exist
    ///   an unassigned binding index `n` such that `0 ≤ n ≤ N`.
    ///
    fn build(&mut self) -> Result<RootSigRef>;
}

/// A builder object for argument pool objects.
pub type ArgPoolBuilderRef = Box<dyn ArgPoolBuilder>;

/// Trait for building argument pool objects.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, arg_table_sig: &ArgTableSigRef) {
///     let arg_pool = device.build_arg_pool()
///         .reserve_table_sig(64, arg_table_sig)
///         .build()
///         .expect("Failed to create an argument pool.");
///     # }
///
pub trait ArgPoolBuilder: Object {
    /// Specify the queue associated with the created argument pool.
    ///
    /// Defaults to the backend-specific value.
    fn queue(&mut self, queue: &CmdQueueRef) -> &mut dyn ArgPoolBuilder;

    /// Increase the capacity of the created argument pool to contain additional
    /// `count` argument tables of the signature `table`.
    fn reserve_table_sig(
        &mut self,
        count: usize,
        table: &ArgTableSigRef,
    ) -> &mut dyn ArgPoolBuilder;

    /// Increase the capacity of the created argument pool to contain additional
    /// `count` arguments of the type `ty`.
    fn reserve_arg(&mut self, count: usize, ty: ArgType) -> &mut dyn ArgPoolBuilder;

    /// Increase the capacity of the created argument pool to contain additional
    /// `count` argument tables. Does not allocate space for their contents,
    /// which must be done by `reserve_arg`.
    fn reserve_table(&mut self, count: usize) -> &mut dyn ArgPoolBuilder;

    /// Enable [`ArgPool::destroy_tables`].
    ///
    /// [`ArgPool::destroy_tables`]: ArgPool::destroy_tables
    fn enable_destroy_tables(&mut self) -> &mut dyn ArgPoolBuilder;

    /// Build an `ArgPoolRef`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<ArgPoolRef>;
}

/// An argument pool object.
pub type ArgPoolRef = Arc<dyn ArgPool>;

/// Trait for argument pool objects.
///
/// The lifetime of the underlying pool object is associated with that of
/// `ArgPool`. Drop the `ArgPool` to destroy the associated argument pool object.
///
/// All argument tables allocated from a pool are implictly destroyed when
/// the pool is destroyed or resetted.
///
/// # Valid Usage
///
///  - When `ArgTableRef`s are destroyed upon the destruction or the reset
///    operation of the `ArgPool`, the valid usage of `destroy_tables` must be
///    followed.
///
pub trait ArgPool: Object {
    /// Create a proxy object to use this argument pool from a specified queue.
    ///
    /// The default implementation panics with a message indicating that the
    /// backend does not support inter-queue operation.
    fn make_proxy(&self, queue: &CmdQueueRef) -> ArgPoolRef {
        let _ = queue;
        panic!("Inter-queue operation is not supported by this backend.");
    }

    /// Allocate zero or more `ArgTableRef`s from the pool.
    ///
    /// Returns `Ok(Some(vec))` with `vec.len() == count` if the allocation
    /// succeds. Returns `Ok(None)` if the allocation fails due to lack of space.
    fn new_tables(&self, count: usize, table: &ArgTableSigRef) -> Result<Option<Vec<ArgTableRef>>>;

    /// Allocate an `ArgTableRef` from the pool.
    fn new_table(&self, table: &ArgTableSigRef) -> Result<Option<ArgTableRef>> {
        let result = self.new_tables(1, table)?;
        if let Some(mut vec) = result {
            assert_eq!(vec.len(), 1);
            Ok(vec.pop())
        } else {
            Ok(None)
        }
    }

    /// Deallocate zero or more `ArgTableRef`s from the pool.
    ///
    /// # Valid Usage
    ///
    ///  - All of the specified `ArgTableRef`s must originate from this pool.
    ///  - All commands referring to any of the specified `ArgTableRef`s must have
    ///    their execution completed at the point of the call to this method.
    ///  - `destroy_tables` must be enabled on this pool via
    ///     [`ArgPoolBuilder::enable_destroy_tables`].
    ///
    /// [`ArgPoolBuilder::enable_destroy_tables`]: ArgPoolBuilder::enable_destroy_tables
    ///
    fn destroy_tables(&self, tables: &[&ArgTableRef]) -> Result<()>;

    /// Deallocate all `ArgTableRef`s.
    ///
    /// # Valid Usage
    ///
    /// See `destroy_tables`, with the exception that enabling `destroy_tables`
    /// via `ArgPoolBuilder` is not required for this method.
    fn reset(&self) -> Result<()>;
}
