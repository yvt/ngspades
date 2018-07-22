//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for argument table objects, argument table signature objects, and
//! root signature objects, and other relevant types.
use std::sync::Arc;

use crate::command::CmdQueue;
use crate::resources::ImageAspect;
use crate::shader::ShaderStageFlags;
use crate::{ArgArrayIndex, ArgIndex, ArgTableIndex};
use crate::{Object, Result};

define_handle! {
    /// Argument set signature handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    ArgTableSig
}

define_handle! {
    /// Argument set handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    ArgTable
}

define_handle! {
    /// Root signature handle.
    ///
    /// See [the module-level documentation of `handles`](../handles/index.html)
    /// for the generic usage of handles.
    RootSig
}

/// A builder object for argument table signature objects.
pub type ArgTableSigBuilder = Box<dyn ArgTableSigBuilderTrait>;

/// Trait for building argument table signature objects.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device) {
///     let mut builder = device.build_arg_table_sig();
///     builder.arg(0, ArgType::SampledImage)
///         .set_stages(ShaderStage::Fragment.into())
///         .set_len(16);
///     builder.arg(1, ArgType::Sampler)
///         .set_stages(ShaderStage::Fragment.into());
///     builder.arg(2, ArgType::StorageBuffer);
///
///     let arg_table_sig = builder.build()
///         .expect("Failed to create an argument table signature.");
///     # }
///
pub trait ArgTableSigBuilderTrait: Object {
    /// Define an argument. Use the returned `dyn ArgSigTrait` to specify
    /// additional properties of it.
    fn arg(&mut self, index: ArgIndex, ty: ArgType) -> &mut dyn ArgSigTrait;

    /// Build an `ArgTableSig`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<ArgTableSig>;
}

/// Trait for setting properties of an argument in an argument table signature.
pub trait ArgSigTrait: Object {
    /// Set the number of elements. Must be non-zero.
    ///
    /// Defaults to `1`.
    fn set_len(&mut self, x: ArgArrayIndex) -> &mut ArgSigTrait;

    /// Set the set of shader stages from which this argument is used.
    ///
    /// Defaults to all shader stages supported by the backend.
    fn set_stages(&mut self, x: ShaderStageFlags) -> &mut ArgSigTrait;

    /// Set the image aspect.
    ///
    /// Defaults to `Color`. Must be `Color` or `Depth`.
    fn set_image_aspect(&mut self, _: ImageAspect) -> &mut ArgSigTrait;
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
pub type RootSigBuilder = Box<dyn RootSigBuilderTrait>;

/// Trait for building root signature objects.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, arg_table_sig: &ArgTableSig) {
///     let root_sig = device.build_root_sig()
///         .arg_table(0, arg_table_sig)
///         .build()
///         .expect("Failed to create a root signature.");
///     # }
///
pub trait RootSigBuilderTrait: Object {
    /// Set the argument table signature at the specified location.
    fn arg_table(&mut self, index: ArgTableIndex, x: &ArgTableSig) -> &mut dyn RootSigBuilderTrait;

    /// Build an `RootSig`.
    ///
    /// # Valid Usage
    ///
    /// - All mandatory properties must have their values set before this method
    ///   is called.
    /// - Binding indices of argument table signatures must be tightly arranged.
    ///   That is, when `N` is max(binding indices ∪ -1), there must not exist
    ///   an unassigned binding index `n` such that `0 ≤ n ≤ N`.
    ///
    fn build(&mut self) -> Result<RootSig>;
}

/// A builder object for argument pool objects.
pub type ArgPoolBuilder = Box<dyn ArgPoolBuilderTrait>;

/// Trait for building argument pool objects.
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device, arg_table_sig: &ArgTableSig) {
///     let arg_pool = device.build_arg_pool()
///         .reserve_table_sig(64, arg_table_sig)
///         .build()
///         .expect("Failed to create an argument pool.");
///     # }
///
pub trait ArgPoolBuilderTrait: Object {
    /// Specify the queue associated with the created argument pool.
    ///
    /// Defaults to the backend-specific value.
    fn queue(&mut self, queue: &CmdQueue) -> &mut ArgPoolBuilderTrait;

    /// Increase the capacity of the created argument pool to contain additional
    /// `count` argument tables of the signature `table`.
    fn reserve_table_sig(&mut self, count: usize, table: &ArgTableSig) -> &mut ArgPoolBuilderTrait;

    /// Increase the capacity of the created argument pool to contain additional
    /// `count` arguments of the type `ty`.
    fn reserve_arg(&mut self, count: usize, ty: ArgType) -> &mut ArgPoolBuilderTrait;

    /// Increase the capacity of the created argument pool to contain additional
    /// `count` argument tables. Does not allocate space for their contents,
    /// which must be done by `reserve_arg`.
    fn reserve_table(&mut self, count: usize) -> &mut ArgPoolBuilderTrait;

    /// Enable [`ArgPoolTrait::destroy_tables`].
    ///
    /// [`ArgPoolTrait::destroy_tables`]: ArgPoolTrait::destroy_tables
    fn enable_destroy_tables(&mut self) -> &mut ArgPoolBuilderTrait;

    /// Build an `ArgPoolTrait`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<ArgPool>;
}

/// An argument pool object.
pub type ArgPool = Arc<dyn ArgPoolTrait>;

/// Trait for argument pool objects.
///
/// The lifetime of the underlying pool object is associated with that of
/// `ArgPoolTrait`. Drop the `ArgPoolTrait` to destroy the associated argument pool object.
///
/// All argument tables allocated from a pool are implictly destroyed when
/// the pool is destroyed or resetted.
///
/// # Valid Usage
///
///  - When `ArgTable`s are destroyed upon the destruction or the reset
///    operation of the `ArgPoolTrait`, the valid usage of `destroy_tables` must be
///    followed.
///
pub trait ArgPoolTrait: Object {
    /// Create a proxy object to use this argument pool from a specified queue.
    fn make_proxy(&self, queue: &CmdQueue) -> ArgPoolTrait;

    /// Allocate zero or more `ArgTable`s from the pool.
    ///
    /// Returns `Ok(Some(vec))` with `vec.len() == count` if the allocation
    /// succeds. Returns `Ok(None)` if the allocation fails due to lack of space.
    fn new_tables(&self, count: usize, table: &ArgTableSig) -> Result<Option<Vec<ArgTable>>>;

    /// Allocate an `ArgTable` from the pool.
    fn new_table(&self, table: &ArgTableSig) -> Result<Option<ArgTable>> {
        let result = self.new_tables(1, table)?;
        if let Some(mut vec) = result {
            assert_eq!(vec.len(), 1);
            Ok(vec.pop())
        } else {
            Ok(None)
        }
    }

    /// Deallocate zero or more `ArgTable`s from the pool.
    ///
    /// # Valid Usage
    ///
    ///  - All of the specified `ArgTable`s must originate from this pool.
    ///  - All commands referring to any of the specified `ArgTable`s must have
    ///    their execution completed at the point of the call to this method.
    ///  - `destroy_tables` must be enabled on this pool via
    ///     [`ArgPoolBuilderTrait::enable_destroy_tables`].
    ///
    /// [`ArgPoolBuilderTrait::enable_destroy_tables`]: ArgPoolBuilderTrait::enable_destroy_tables
    ///
    fn destroy_tables(&self, tables: &[&ArgTable]) -> Result<()>;

    /// Deallocate all `ArgTable`s.
    ///
    /// # Valid Usage
    ///
    /// See `destroy_tables`, with the exception that enabling `destroy_tables`
    /// via `ArgPoolBuilderTrait` is not required for this method.
    fn reset(&self) -> Result<()>;
}
