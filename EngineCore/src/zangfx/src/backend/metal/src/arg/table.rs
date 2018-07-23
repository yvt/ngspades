//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `ArgPool` and `ArgTable` for Metal.
use std::sync::Arc;
use zangfx_metal_rs as metal;

use zangfx_base::Result;
use zangfx_base::{self as base, arg};
use zangfx_base::{interfaces, vtable_for, zangfx_impl_handle, zangfx_impl_object};

use crate::utils::{nil_error, OCPtr};

use super::allocator::{Allocation, Allocator, StackAllocator, TlsfAllocator};
use super::tablesig::ArgTableSig;
use super::ArgSize;

/// Device-specific characteristics of argument buffers' layout.
///
/// The information from this is used to convert the Vulkan-style pool size
/// specification to Metal-style. The conversion might not be perfect but would
/// be a good approximation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ArgLayoutInfo {
    table_size: ArgSize,
    texture_size: ArgSize,
    buffer_size: ArgSize,
    sampler_size: ArgSize,
}

impl ArgLayoutInfo {
    /// Compute the `ArgLayoutInfo` for a given device.
    crate unsafe fn new(metal_device: metal::MTLDevice) -> Result<Self> {
        use zangfx_base::arg::ArgTableSigBuilder;
        let mut builder = super::tablesig::ArgTableSigBuilder::new(metal_device);
        builder.arg(0, arg::ArgType::StorageImage);

        // `table_size + texture_size`
        let arg_size = builder.encoded_size()?;

        // `table_size + texture_size + texture_size`
        let arg_t_size = {
            builder.arg(1, arg::ArgType::StorageImage);
            builder.encoded_size()?
        };

        // `table_size + texture_size + buffer_size`
        let arg_b_size = {
            builder.arg(1, arg::ArgType::StorageBuffer);
            builder.encoded_size()?
        };

        // `table_size + texture_size + sampler_size`
        let arg_s_size = {
            builder.arg(1, arg::ArgType::Sampler);
            builder.encoded_size()?
        };

        Ok(Self {
            table_size: arg_size * 2 - arg_t_size,
            texture_size: arg_t_size - arg_size,
            buffer_size: arg_b_size - arg_size,
            sampler_size: arg_s_size - arg_size,
        })
    }
}

/// Implementation of `ArgPoolBuilder` for Metal.
#[derive(Debug)]
pub struct ArgPoolBuilder {
    /// A reference to a `MTLDevice`. We are not required to maintain a strong
    /// reference. (See the base interface's documentation)
    metal_device: metal::MTLDevice,
    layout: ArgLayoutInfo,

    size: ArgSize,
    enable_destroy_tables: bool,

    label: Option<String>,
}

zangfx_impl_object! { ArgPoolBuilder: dyn arg::ArgPoolBuilder, dyn crate::Debug, dyn base::SetLabel }

unsafe impl Send for ArgPoolBuilder {}
unsafe impl Sync for ArgPoolBuilder {}

impl ArgPoolBuilder {
    /// Construct an `ArgPoolBuilder`.
    ///
    /// Ir's up to the caller to maintain the lifetime of `metal_device`.
    pub(crate) unsafe fn new(metal_device: metal::MTLDevice, layout: ArgLayoutInfo) -> Self {
        Self {
            metal_device,
            layout,
            size: 0,
            enable_destroy_tables: false,
            label: None,
        }
    }
}

impl base::SetLabel for ArgPoolBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
    }
}

impl arg::ArgPoolBuilder for ArgPoolBuilder {
    fn queue(&mut self, _queue: &base::CmdQueueRef) -> &mut dyn base::ArgPoolBuilder {
        self
    }

    fn reserve_table_sig(
        &mut self,
        count: usize,
        table: &arg::ArgTableSigRef,
    ) -> &mut dyn arg::ArgPoolBuilder {
        if count == 0 {
            return self;
        }

        let count = count as ArgSize;

        let our_table: &ArgTableSig = table
            .downcast_ref()
            .expect("bad argument table signature type");
        let (size, align) = (our_table.encoded_size(), our_table.encoded_alignment());

        self.size = (self.size + align - 1) & !(align - 1);

        self.size += ((size + align - 1) & !(align - 1)) * count;

        self
    }

    fn reserve_arg(&mut self, count: usize, ty: arg::ArgType) -> &mut dyn arg::ArgPoolBuilder {
        use zangfx_base::arg::ArgType::*;
        self.size += match ty {
            StorageImage | SampledImage => self.layout.texture_size,
            Sampler => self.layout.sampler_size,
            UniformBuffer | StorageBuffer => self.layout.buffer_size,
        } * count as ArgSize;
        self
    }

    fn reserve_table(&mut self, count: usize) -> &mut dyn arg::ArgPoolBuilder {
        self.size += self.layout.table_size * count as ArgSize;
        self
    }

    fn enable_destroy_tables(&mut self) -> &mut dyn arg::ArgPoolBuilder {
        self.enable_destroy_tables = true;
        self
    }

    fn build(&mut self) -> Result<arg::ArgPoolRef> {
        if self.size == 0 {
            return Ok(Arc::new(ZeroSizedArgPool));
        }

        // Allocate a buffer for the newly created argument pool
        let metal_buffer = {
            let options =
                metal::MTLResourceStorageModeShared | metal::MTLResourceHazardTrackingModeUntracked;
            unsafe { OCPtr::from_raw(self.metal_device.new_buffer(self.size as _, options)) }
                .ok_or_else(|| nil_error("MTLDevice newBufferWithLength:options:"))?
        };

        if let Some(ref label) = self.label {
            metal_buffer.set_label(label);
        }

        if self.enable_destroy_tables {
            Ok(Arc::new(DynamicArgPool(BaseArgPool::new(metal_buffer))))
        } else {
            Ok(Arc::new(StackArgPool(BaseArgPool::new(metal_buffer))))
        }
    }
}

/// Generic implementation of `ArgPool` for Metal.
/// (Because `zangfx_impl_object` does not support generics)
#[derive(Debug)]
struct BaseArgPool<T> {
    metal_buffer: OCPtr<metal::MTLBuffer>,
    allocator: T,
}

unsafe impl<T> Send for BaseArgPool<T> {}
unsafe impl<T> Sync for BaseArgPool<T> {}

impl<T: Allocator> BaseArgPool<T> {
    fn new(metal_buffer: OCPtr<metal::MTLBuffer>) -> Self {
        let size = metal_buffer.length() as ArgSize;
        Self {
            metal_buffer,
            allocator: T::new(size),
        }
    }

    fn new_tables(
        &mut self,
        count: usize,
        table: &arg::ArgTableSigRef,
    ) -> Result<Option<Vec<arg::ArgTableRef>>> {
        let our_sig: &ArgTableSig = table
            .downcast_ref()
            .expect("bad argument table signature type");
        let (size, align) = (our_sig.encoded_size(), our_sig.encoded_alignment());

        let mut alloc_infos = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(alloc_info) = self.allocator.allocate(size, align) {
                alloc_infos.push(alloc_info);
            } else {
                break;
            }
        }

        if alloc_infos.len() < count {
            // Allocation has failed -- rollback
            for (_, alloc) in alloc_infos {
                self.allocator.deallocate(alloc);
            }
            return Ok(None);
        }

        let tables = alloc_infos
            .into_iter()
            .map(|(offset, allocation)| {
                let our_table = ArgTable {
                    metal_buffer: *self.metal_buffer,
                    offset,
                    allocation,
                };
                arg::ArgTableRef::new(our_table)
            })
            .collect();
        Ok(Some(tables))
    }

    fn destroy_tables(&mut self, tables: &[&arg::ArgTableRef]) -> Result<()> {
        for table in tables.iter() {
            let our_table: &ArgTable = table.downcast_ref().expect("bad argument table type");
            self.allocator.deallocate(our_table.clone().allocation);
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.allocator.reset();
        Ok(())
    }
}

/// Implementation of `ArgPool` for Metal. Employs the stack-based dynamic
/// allocator and does not support deallocation (except for resetting).
#[derive(Debug)]
pub struct StackArgPool(BaseArgPool<StackAllocator>);

zangfx_impl_object! { StackArgPool: dyn arg::ArgPool, dyn crate::Debug }

impl arg::ArgPool for StackArgPool {
    fn new_tables(
        &self,
        count: usize,
        table: &arg::ArgTableSigRef,
    ) -> Result<Option<Vec<arg::ArgTableRef>>> {
        unimplemented!() // self.0.new_tables(count, table)
    }

    fn destroy_tables(&self, tables: &[&arg::ArgTableRef]) -> Result<()> {
        unimplemented!() // self.0.destroy_tables(tables)
    }

    fn reset(&self) -> Result<()> {
        unimplemented!() // self.0.reset()
    }
}

/// Implementation of `ArgPool` for Metal. Employs the full dynamic allocator.
#[derive(Debug)]
pub struct DynamicArgPool(BaseArgPool<TlsfAllocator>);

zangfx_impl_object! { DynamicArgPool: dyn arg::ArgPool, dyn crate::Debug }

impl arg::ArgPool for DynamicArgPool {
    fn new_tables(
        &self,
        count: usize,
        table: &arg::ArgTableSigRef,
    ) -> Result<Option<Vec<arg::ArgTableRef>>> {
        unimplemented!() // self.0.new_tables(count, table)
    }

    fn destroy_tables(&self, tables: &[&arg::ArgTableRef]) -> Result<()> {
        unimplemented!() // self.0.destroy_tables(tables)
    }

    fn reset(&self) -> Result<()> {
        unimplemented!() // self.0.reset()
    }
}

/// Implementation of `ArgPool` for Metal. Size is zero.
#[derive(Debug)]
pub struct ZeroSizedArgPool;

zangfx_impl_object! { ZeroSizedArgPool: dyn arg::ArgPool, dyn crate::Debug }

impl arg::ArgPool for ZeroSizedArgPool {
    fn new_tables(
        &self,
        _count: usize,
        _table: &arg::ArgTableSigRef,
    ) -> Result<Option<Vec<arg::ArgTableRef>>> {
        Ok(None)
    }

    fn destroy_tables(&self, _: &[&arg::ArgTableRef]) -> Result<()> {
        panic!("ZeroSizedArgPool does not support allocation at all")
    }

    fn reset(&self) -> Result<()> {
        Ok(())
    }
}

/// Implementation of `ArgTable` for Metal.
#[derive(Debug)]
pub struct ArgTable {
    metal_buffer: metal::MTLBuffer,
    offset: ArgSize,
    allocation: Allocation,
}

zangfx_impl_handle! { ArgTable, arg::ArgTableRef }

unsafe impl Send for ArgTable {}
unsafe impl Sync for ArgTable {}

impl Clone for ArgTable {
    fn clone(&self) -> ArgTable {
        use std::mem::transmute_copy;
        ArgTable {
            metal_buffer: self.metal_buffer,
            offset: self.offset,
            // `Allocation` is not `Clone`, but this is safe as long as the
            // application follows the valid usage of `ArgPool`. (Specifically,
            // the application must not call `destroy_tables` twice)
            allocation: unsafe { transmute_copy(&self.allocation) },
        }
    }
}

impl ArgTable {
    pub unsafe fn from_raw(metal_buffer: metal::MTLBuffer, offset: ArgSize) -> Self {
        Self {
            metal_buffer,
            offset,
            allocation: None,
        }
    }

    pub fn metal_buffer(&self) -> metal::MTLBuffer {
        self.metal_buffer
    }

    pub fn offset(&self) -> ArgSize {
        self.offset
    }
}
