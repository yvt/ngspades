//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;

use zangfx::base as gfx;
use self::gfx::Result;

use gfxutils::{MultiHeapSet, MultiHeapSetAlloc};

/// Temporary resource pool (duh).
#[derive(Debug)]
pub struct TempResPool {
    device: Arc<gfx::Device>,
    heap: MultiHeapSet,
}

#[derive(Debug, Default)]
pub struct TempResTable {
    allocs: Vec<MultiHeapSetAlloc>,
    images: Vec<gfx::Image>,
    image_views: Vec<gfx::ImageView>,
    buffers: Vec<gfx::Buffer>,
}

impl TempResPool {
    pub fn new(device: Arc<gfx::Device>) -> Result<Self> {
        let heap = MultiHeapSet::new(&device);
        Ok(Self { device, heap })
    }

    pub fn heap_mut(&mut self) -> &mut MultiHeapSet {
        &mut self.heap
    }

    /// Construct a `TempResTable` associated with this `TempResPool`.
    pub fn new_table(&self) -> TempResTable {
        Default::default()
    }

    /// Release temporary resources.
    pub fn release(&mut self, table: &mut TempResTable) -> Result<()> {
        for alloc in table.allocs.drain(..) {
            self.heap.unbind(&alloc)?;
        }
        for image in table.images.drain(..) {
            self.device.destroy_image(&image)?;
        }
        for image_view in table.image_views.drain(..) {
            self.device.destroy_image_view(&image_view)?;
        }
        for buffer in table.buffers.drain(..) {
            self.device.destroy_buffer(&buffer)?;
        }
        Ok(())
    }

    pub fn bind<'a, T: Into<gfx::ResourceRef<'a>>>(
        &mut self,
        table: &mut TempResTable,
        memory_type: gfx::MemoryType,
        resource: T,
    ) -> Result<MultiHeapSetAlloc> {
        table.allocs.reserve(1);
        let alloc = self.heap.bind_dynamic(memory_type, resource)?;
        table.allocs.push(alloc.clone());
        Ok(alloc)
    }

    pub fn as_ptr(&self, alloc: &MultiHeapSetAlloc) -> Result<*mut u8> {
        self.heap.as_ptr(&alloc)
    }

    pub fn add_buffer(&mut self, table: &mut TempResTable, buffer: gfx::Buffer) {
        table.buffers.push(buffer);
        table.buffers.reserve(1);
    }

    pub fn add_image(&mut self, table: &mut TempResTable, image: gfx::Image) {
        table.images.push(image);
        table.images.reserve(1);
    }

    pub fn add_image_view(&mut self, table: &mut TempResTable, image_view: gfx::ImageView) {
        table.image_views.push(image_view);
        table.image_views.reserve(1);
    }
}
