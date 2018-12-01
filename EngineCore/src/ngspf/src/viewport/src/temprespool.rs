//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;

use self::gfx::Result;
use zangfx::base as gfx;

use crate::gfxutils::{MultiHeapSet, MultiHeapSetAlloc};

/// Temporary resource pool (duh).
#[derive(Debug)]
pub struct TempResPool {
    device: Arc<gfx::Device>,
    heap: MultiHeapSet,
}

#[derive(Debug, Default)]
pub struct TempResTable {
    allocs: Vec<(Resource, MultiHeapSetAlloc)>,
}

#[derive(Debug, Clone)]
enum Resource {
    Image(gfx::ImageRef),
    Buffer(gfx::BufferRef),
}

impl Resource {
    fn clone_from(x: gfx::ResourceRef<'_>) -> Self {
        match x {
            gfx::ResourceRef::Image(x) => Resource::Image(x.clone()),
            gfx::ResourceRef::Buffer(x) => Resource::Buffer(x.clone()),
        }
    }

    fn as_ref(&self) -> gfx::ResourceRef<'_> {
        match self {
            Resource::Image(ref x) => gfx::ResourceRef::Image(x),
            Resource::Buffer(ref x) => gfx::ResourceRef::Buffer(x),
        }
    }
}

impl TempResPool {
    pub fn new(device: gfx::DeviceRef) -> Result<Self> {
        let heap = MultiHeapSet::new(&device);
        Ok(Self { device, heap })
    }

    /// Construct a `TempResTable` associated with this `TempResPool`.
    pub fn new_table(&self) -> TempResTable {
        Default::default()
    }

    /// Release temporary resources.
    pub fn release(&mut self, table: &mut TempResTable) -> Result<()> {
        for (resource, alloc) in table.allocs.drain(..) {
            self.heap.unbind(&alloc, resource.as_ref())?;
        }
        Ok(())
    }

    pub fn bind<'a, T: Into<gfx::ResourceRef<'a>>>(
        &mut self,
        table: &mut TempResTable,
        memory_type: gfx::MemoryType,
        resource: T,
    ) -> Result<MultiHeapSetAlloc> {
        let resource = resource.into();
        table.allocs.reserve(1);
        let alloc = self.heap.bind_dynamic(memory_type, resource)?;
        table.allocs.push((Resource::clone_from(resource), alloc));
        Ok(alloc)
    }
}
